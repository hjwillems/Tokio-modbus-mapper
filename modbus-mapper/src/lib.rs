//! # Modbus Mapper
//!
//! A zero-cost procedural macro crate for mapping Rust types to Modbus registers.
//!
//! This crate provides a thin, lightweight layer on top of `tokio-modbus` that automatically
//! generates type-safe serialization and deserialization code for Modbus register mappings.
//!
//! ## Features
//!
//! - **Comprehensive type support**: primitives, strings, arrays, Option, enums, bit fields, nested structs, tuples
//! - **Zero runtime overhead**: All code generated at compile time
//! - **Client and server modes**: Read/write from devices or respond to requests
//! - **Configurable endianness**: Per-field big/little-endian word order
//! - **Compile-time validation**: Catch mapping errors before running
//!
//! ## Example
//!
//! ```ignore
//! use modbus_mapper::ModbusMapper;
//!
//! #[derive(ModbusMapper)]
//! #[modbus(base_address = 0, register_type = "holding")]
//! struct SensorData {
//!     #[modbus(address = 0)]
//!     temperature: f32,
//!
//!     #[modbus(address = 2)]
//!     pressure: u16,
//!
//!     #[modbus(address = 3)]
//!     status_flags: u16,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut ctx = tokio_modbus::client::tcp::connect("192.168.1.100:502").await?;
//!
//!     // Read entire struct from Modbus device
//!     let data = SensorData::read_from_modbus(&mut ctx).await?;
//!     println!("Temperature: {}", data.temperature);
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// Re-export the derive macro
pub use modbus_mapper_derive::{ModbusEnum, ModbusMapper};

// Re-export tokio-modbus types that users will need
pub use tokio_modbus;

// Public modules
pub mod endian;
pub mod error;

// Re-export commonly used types
pub use endian::Endianness;
pub use error::{ModbusMapperError, Result};

/// Register type for Modbus operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterType {
    /// Holding registers (read/write, 16-bit).
    Holding,
    /// Input registers (read-only, 16-bit).
    Input,
    /// Coils (read/write, 1-bit).
    Coil,
    /// Discrete inputs (read-only, 1-bit).
    Discrete,
}

impl RegisterType {
    /// Returns true if this register type is writable.
    pub fn is_writable(self) -> bool {
        matches!(self, Self::Holding | Self::Coil)
    }

    /// Returns true if this register type is read-only.
    pub fn is_readonly(self) -> bool {
        !self.is_writable()
    }

    /// Returns the name of the register type as a string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Holding => "holding",
            Self::Input => "input",
            Self::Coil => "coil",
            Self::Discrete => "discrete",
        }
    }
}

/// Trait for types that can be serialized to Modbus registers.
///
/// This trait is automatically implemented by the `#[derive(ModbusMapper)]` macro.
pub trait ToRegisters {
    /// Convert the value to a vector of 16-bit registers.
    fn to_registers(&self) -> Vec<u16>;

    /// Get the number of registers required to store this type.
    fn register_count() -> u16;
}

/// Trait for types that can be deserialized from Modbus registers.
///
/// This trait is automatically implemented by the `#[derive(ModbusMapper)]` macro.
pub trait FromRegisters: Sized {
    /// Create a value from a slice of 16-bit registers.
    ///
    /// # Errors
    ///
    /// Returns an error if the register count doesn't match or if the data is invalid.
    fn from_registers(registers: &[u16]) -> Result<Self>;
}

/// Metadata about a Modbus-mapped struct.
///
/// This trait is automatically implemented by the `#[derive(ModbusMapper)]` macro.
pub trait ModbusMetadata {
    /// Get the base address for this struct.
    fn base_address() -> u16;

    /// Get the register type for this struct.
    fn register_type() -> RegisterType;

    /// Get the register address for a specific field by name.
    ///
    /// Returns `None` if the field doesn't exist or is skipped.
    fn field_address(field_name: &str) -> Option<u16>;

    /// Get the register count for a specific field by name.
    ///
    /// Returns `None` if the field doesn't exist or is skipped.
    fn field_register_count(field_name: &str) -> Option<u16>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_type() {
        assert!(RegisterType::Holding.is_writable());
        assert!(RegisterType::Coil.is_writable());
        assert!(RegisterType::Input.is_readonly());
        assert!(RegisterType::Discrete.is_readonly());

        assert_eq!(RegisterType::Holding.as_str(), "holding");
        assert_eq!(RegisterType::Input.as_str(), "input");
    }
}
