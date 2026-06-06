//! Async client integration with `tokio-modbus`.
//!
//! This module is the thin layer that gives the crate its name: it connects the
//! compile-time-generated [`ToRegisters`]/[`FromRegisters`]/[`ModbusMetadata`]
//! implementations to real `tokio-modbus` I/O.
//!
//! Two extension traits are provided and blanket-implemented for every mapped
//! type, so any `#[derive(ModbusMapper)]` struct gains `read_from_modbus` /
//! `write_to_modbus` for free:
//!
//! ```no_run
//! use modbus_mapper::{ModbusMapper, ModbusRead, ModbusWrite};
//! use tokio_modbus::prelude::*;
//!
//! #[derive(ModbusMapper)]
//! #[modbus(base_address = 0, register_type = "holding")]
//! struct SensorData {
//!     #[modbus(address = 0)]
//!     temperature: f32,
//!     #[modbus(address = 2)]
//!     pressure: u16,
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let mut ctx = tcp::connect("192.168.1.100:502".parse().unwrap()).await?;
//!
//! // Read the whole block in one request, decode into the struct.
//! let data = SensorData::read_from_modbus(&mut ctx).await?;
//!
//! // Encode the struct and write the whole block in one request.
//! data.write_to_modbus(&mut ctx).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use tokio_modbus::client::{Reader, Writer};

use crate::{FromRegisters, ModbusMapperError, ModbusMetadata, RegisterType, Result, ToRegisters};

/// Read a mapped struct from a Modbus device.
///
/// Blanket-implemented for every type that is [`FromRegisters`] + [`ModbusMetadata`],
/// i.e. every `#[derive(ModbusMapper)]` struct.
///
/// The read is issued at the struct's [`base_address`](ModbusMetadata::base_address)
/// for exactly [`register_count`](ToRegisters::register_count) registers, using the
/// function code implied by the struct's [`RegisterType`].
#[async_trait]
pub trait ModbusRead: Sized {
    /// Read and decode the entire register block for this type in a single request.
    async fn read_from_modbus<R>(ctx: &mut R) -> Result<Self>
    where
        R: Reader + Send;
}

// `register_count()` lives on `ToRegisters`; every `#[derive(ModbusMapper)]` type
// implements all three traits, so requiring it here costs nothing in practice.
#[async_trait]
impl<T> ModbusRead for T
where
    T: FromRegisters + ToRegisters + ModbusMetadata + Send,
{
    async fn read_from_modbus<R>(ctx: &mut R) -> Result<Self>
    where
        R: Reader + Send,
    {
        let address = T::base_address();
        let count = T::register_count();

        let words = match T::register_type() {
            RegisterType::Holding => ctx.read_holding_registers(address, count).await,
            RegisterType::Input => ctx.read_input_registers(address, count).await,
            register_type => {
                return Err(ModbusMapperError::UnsupportedRegisterType {
                    register_type,
                    operation: "read",
                })
            }
        }??;

        T::from_registers(&words)
    }
}

/// Write a mapped struct to a Modbus device.
///
/// Blanket-implemented for every type that is [`ToRegisters`] + [`ModbusMetadata`],
/// i.e. every `#[derive(ModbusMapper)]` struct.
///
/// The write is issued at the struct's [`base_address`](ModbusMetadata::base_address)
/// using `Write Multiple Registers` (0x10). Read-only register types (`input`)
/// return [`ModbusMapperError::UnsupportedRegisterType`].
#[async_trait]
pub trait ModbusWrite {
    /// Encode and write the entire register block for this value in a single request.
    async fn write_to_modbus<W>(&self, ctx: &mut W) -> Result<()>
    where
        W: Writer + Send;
}

#[async_trait]
impl<T> ModbusWrite for T
where
    T: ToRegisters + ModbusMetadata + Sync,
{
    async fn write_to_modbus<W>(&self, ctx: &mut W) -> Result<()>
    where
        W: Writer + Send,
    {
        let register_type = T::register_type();
        if !register_type.is_writable() {
            return Err(ModbusMapperError::UnsupportedRegisterType {
                register_type,
                operation: "write",
            });
        }

        match register_type {
            RegisterType::Holding => {
                let words = self.to_registers();
                ctx.write_multiple_registers(T::base_address(), &words)
                    .await??;
                Ok(())
            }
            // Coil/Discrete are rejected at derive time; guard here for hand-written impls.
            register_type => Err(ModbusMapperError::UnsupportedRegisterType {
                register_type,
                operation: "write",
            }),
        }
    }
}
