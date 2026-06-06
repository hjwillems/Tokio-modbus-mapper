//! Integration tests for the async `tokio-modbus` client layer.
//!
//! These exercise the real `ModbusRead`/`ModbusWrite` code path against an
//! in-memory mock device. The mock implements `tokio_modbus::client::Client`
//! and is wrapped in a genuine `tokio_modbus::client::Context`, so the same
//! `Reader`/`Writer` implementations used against real hardware are tested here.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use modbus_mapper::{ModbusMapper, ModbusMapperError, ModbusRead, ModbusWrite};
use tokio_modbus::client::{Client, Context};
use tokio_modbus::slave::SlaveContext;
use tokio_modbus::{Request, Response, Slave};

/// Shared register space so a test can inspect exactly where writes landed.
type Registers = Arc<Mutex<Vec<u16>>>;

#[derive(Debug)]
struct MockDevice {
    registers: Registers,
}

impl SlaveContext for MockDevice {
    fn set_slave(&mut self, _slave: Slave) {}
}

#[async_trait]
impl Client for MockDevice {
    async fn call(&mut self, request: Request<'_>) -> tokio_modbus::Result<Response> {
        let response = match request {
            Request::ReadHoldingRegisters(addr, cnt) => {
                let regs = self.registers.lock().unwrap();
                let (a, c) = (addr as usize, cnt as usize);
                Response::ReadHoldingRegisters(regs[a..a + c].to_vec())
            }
            Request::ReadInputRegisters(addr, cnt) => {
                let regs = self.registers.lock().unwrap();
                let (a, c) = (addr as usize, cnt as usize);
                Response::ReadInputRegisters(regs[a..a + c].to_vec())
            }
            Request::WriteMultipleRegisters(addr, words) => {
                let mut regs = self.registers.lock().unwrap();
                let a = addr as usize;
                for (i, w) in words.iter().enumerate() {
                    regs[a + i] = *w;
                }
                Response::WriteMultipleRegisters(addr, words.len() as u16)
            }
            other => panic!("mock device received unexpected request: {other:?}"),
        };
        Ok(Ok(response))
    }
}

/// Build a `Context` backed by a fresh mock device, returning a handle to its
/// register space for inspection.
fn mock_device() -> (Context, Registers) {
    let registers: Registers = Arc::new(Mutex::new(vec![0u16; 256]));
    let device = MockDevice {
        registers: Arc::clone(&registers),
    };
    let ctx = Context::from(Box::new(device) as Box<dyn Client>);
    (ctx, registers)
}

#[derive(ModbusMapper, Debug, PartialEq)]
#[modbus(base_address = 0, register_type = "holding")]
struct Holding {
    #[modbus(address = 0)]
    temperature: f32,
    #[modbus(address = 2)]
    pressure: u16,
    #[modbus(address = 3)]
    flags: u16,
}

#[derive(ModbusMapper, Debug, PartialEq)]
#[modbus(base_address = 10, register_type = "holding")]
struct OffsetBlock {
    #[modbus(address = 0)]
    x: u16,
    #[modbus(address = 1)]
    y: u16,
}

#[derive(ModbusMapper, Debug, PartialEq)]
#[modbus(base_address = 0, register_type = "input")]
struct InputBlock {
    #[modbus(address = 0)]
    a: u16,
    #[modbus(address = 1)]
    b: u16,
}

#[tokio::test]
async fn write_then_read_roundtrip_holding() {
    let (mut ctx, _regs) = mock_device();

    let original = Holding {
        temperature: 25.5,
        pressure: 1013,
        flags: 0xBEEF,
    };

    original.write_to_modbus(&mut ctx).await.unwrap();
    let read_back = Holding::read_from_modbus(&mut ctx).await.unwrap();

    assert_eq!(read_back, original);
}

#[tokio::test]
async fn write_lands_at_base_address() {
    let (mut ctx, regs) = mock_device();

    let block = OffsetBlock {
        x: 0x1111,
        y: 0x2222,
    };
    block.write_to_modbus(&mut ctx).await.unwrap();

    // base_address = 10, so the two registers must land at wire offsets 10 and 11,
    // and nowhere else.
    let regs = regs.lock().unwrap();
    assert_eq!(regs[10], 0x1111);
    assert_eq!(regs[11], 0x2222);
    assert_eq!(regs[0], 0, "nothing should have been written at offset 0");
}

#[tokio::test]
async fn read_at_base_address() {
    let (mut ctx, regs) = mock_device();
    {
        let mut regs = regs.lock().unwrap();
        regs[10] = 7;
        regs[11] = 9;
    }

    let block = OffsetBlock::read_from_modbus(&mut ctx).await.unwrap();
    assert_eq!(block, OffsetBlock { x: 7, y: 9 });
}

#[tokio::test]
async fn read_input_registers() {
    let (mut ctx, regs) = mock_device();
    {
        let mut regs = regs.lock().unwrap();
        regs[0] = 111;
        regs[1] = 222;
    }

    let block = InputBlock::read_from_modbus(&mut ctx).await.unwrap();
    assert_eq!(block, InputBlock { a: 111, b: 222 });
}

#[tokio::test]
async fn write_to_read_only_input_is_rejected() {
    let (mut ctx, _regs) = mock_device();

    let block = InputBlock { a: 1, b: 2 };
    let err = block.write_to_modbus(&mut ctx).await.unwrap_err();

    assert!(matches!(
        err,
        ModbusMapperError::UnsupportedRegisterType {
            operation: "write",
            ..
        }
    ));
}
