// A gap between field addresses must be rejected at compile time: `b` is declared
// at address 10 but the contiguous layout expects address 1. Accepting this would
// silently produce a 2-register buffer that does not line up with the device.
use modbus_mapper::ModbusMapper;

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct Gappy {
    #[modbus(address = 0)]
    a: u16,
    #[modbus(address = 10)]
    b: u16,
}

fn main() {}
