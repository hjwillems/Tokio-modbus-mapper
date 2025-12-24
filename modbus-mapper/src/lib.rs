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

/// Modbus function codes as defined in the Modbus specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FunctionCode {
    // ===== Bit Access =====
    /// Read Coils (0x01) - Read 1-n bits from coils
    ReadCoils = 0x01,
    /// Read Discrete Inputs (0x02) - Read 1-n bits from discrete inputs
    ReadDiscreteInputs = 0x02,
    /// Write Single Coil (0x05) - Write 1 bit to a coil
    WriteSingleCoil = 0x05,
    /// Write Multiple Coils (0x0F) - Write 1-n bits to coils
    WriteMultipleCoils = 0x0F,

    // ===== 16-bit Register Access =====
    /// Read Holding Registers (0x03) - Read 1-n 16-bit registers from holding area
    ReadHoldingRegisters = 0x03,
    /// Read Input Registers (0x04) - Read 1-n 16-bit registers from input area
    ReadInputRegisters = 0x04,
    /// Write Single Register (0x06) - Write 1 register to holding area
    WriteSingleRegister = 0x06,
    /// Write Multiple Registers (0x10) - Write 1-n registers to holding area
    WriteMultipleRegisters = 0x10,
    /// Mask Write Register (0x16) - Modify bits in a single holding register
    MaskWriteRegister = 0x16,
    /// Read/Write Multiple Registers (0x17) - Combined read/write operation
    ReadWriteMultipleRegisters = 0x17,

    // ===== Extended Functions =====
    /// Read Exception Status (0x07) - Read 8 exception status bits
    ReadExceptionStatus = 0x07,
    /// Diagnostics (0x08) - Diagnostic functions with sub-function codes
    Diagnostics = 0x08,
    /// Get Comm Event Counter (0x0B) - Get communication event counter
    GetCommEventCounter = 0x0B,
    /// Get Comm Event Log (0x0C) - Get communication event log
    GetCommEventLog = 0x0C,
    /// Report Server ID (0x11) - Get server identification
    ReportServerId = 0x11,
    /// Read File Record (0x14) - Read file records
    ReadFileRecord = 0x14,
    /// Write File Record (0x15) - Write file records
    WriteFileRecord = 0x15,
    /// Read FIFO Queue (0x18) - Read from FIFO queue
    ReadFifoQueue = 0x18,
    /// Encapsulated Interface Transport (0x2B) - MEI transport
    EncapsulatedInterfaceTransport = 0x2B,
}

impl FunctionCode {
    /// Create a FunctionCode from a u8 value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::ReadCoils),
            0x02 => Some(Self::ReadDiscreteInputs),
            0x03 => Some(Self::ReadHoldingRegisters),
            0x04 => Some(Self::ReadInputRegisters),
            0x05 => Some(Self::WriteSingleCoil),
            0x06 => Some(Self::WriteSingleRegister),
            0x07 => Some(Self::ReadExceptionStatus),
            0x08 => Some(Self::Diagnostics),
            0x0B => Some(Self::GetCommEventCounter),
            0x0C => Some(Self::GetCommEventLog),
            0x0F => Some(Self::WriteMultipleCoils),
            0x10 => Some(Self::WriteMultipleRegisters),
            0x11 => Some(Self::ReportServerId),
            0x14 => Some(Self::ReadFileRecord),
            0x15 => Some(Self::WriteFileRecord),
            0x16 => Some(Self::MaskWriteRegister),
            0x17 => Some(Self::ReadWriteMultipleRegisters),
            0x18 => Some(Self::ReadFifoQueue),
            0x2B => Some(Self::EncapsulatedInterfaceTransport),
            _ => None,
        }
    }

    /// Convert to u8 value.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns true if this is a read operation.
    pub fn is_read(self) -> bool {
        matches!(
            self,
            Self::ReadCoils
                | Self::ReadDiscreteInputs
                | Self::ReadHoldingRegisters
                | Self::ReadInputRegisters
                | Self::ReadExceptionStatus
                | Self::GetCommEventCounter
                | Self::GetCommEventLog
                | Self::ReportServerId
                | Self::ReadFileRecord
                | Self::ReadFifoQueue
                | Self::ReadWriteMultipleRegisters
        )
    }

    /// Returns true if this is a write operation.
    pub fn is_write(self) -> bool {
        matches!(
            self,
            Self::WriteSingleCoil
                | Self::WriteMultipleCoils
                | Self::WriteSingleRegister
                | Self::WriteMultipleRegisters
                | Self::MaskWriteRegister
                | Self::WriteFileRecord
                | Self::ReadWriteMultipleRegisters
        )
    }

    /// Returns true if this function operates on bits (coils/discrete inputs).
    pub fn is_bit_access(self) -> bool {
        matches!(
            self,
            Self::ReadCoils
                | Self::ReadDiscreteInputs
                | Self::WriteSingleCoil
                | Self::WriteMultipleCoils
        )
    }

    /// Returns true if this function operates on 16-bit registers.
    pub fn is_register_access(self) -> bool {
        matches!(
            self,
            Self::ReadHoldingRegisters
                | Self::ReadInputRegisters
                | Self::WriteSingleRegister
                | Self::WriteMultipleRegisters
                | Self::MaskWriteRegister
                | Self::ReadWriteMultipleRegisters
        )
    }

    /// Get the function name as a string.
    pub fn name(self) -> &'static str {
        match self {
            Self::ReadCoils => "Read Coils",
            Self::ReadDiscreteInputs => "Read Discrete Inputs",
            Self::ReadHoldingRegisters => "Read Holding Registers",
            Self::ReadInputRegisters => "Read Input Registers",
            Self::WriteSingleCoil => "Write Single Coil",
            Self::WriteSingleRegister => "Write Single Register",
            Self::WriteMultipleCoils => "Write Multiple Coils",
            Self::WriteMultipleRegisters => "Write Multiple Registers",
            Self::MaskWriteRegister => "Mask Write Register",
            Self::ReadWriteMultipleRegisters => "Read/Write Multiple Registers",
            Self::ReadExceptionStatus => "Read Exception Status",
            Self::Diagnostics => "Diagnostics",
            Self::GetCommEventCounter => "Get Comm Event Counter",
            Self::GetCommEventLog => "Get Comm Event Log",
            Self::ReportServerId => "Report Server ID",
            Self::ReadFileRecord => "Read File Record",
            Self::WriteFileRecord => "Write File Record",
            Self::ReadFifoQueue => "Read FIFO Queue",
            Self::EncapsulatedInterfaceTransport => "Encapsulated Interface Transport",
        }
    }
}

/// Register type for Modbus operations.
///
/// This is a higher-level abstraction over function codes for common use cases.
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

    /// Get the default read function code for this register type.
    pub fn read_function_code(self) -> FunctionCode {
        match self {
            Self::Holding => FunctionCode::ReadHoldingRegisters,
            Self::Input => FunctionCode::ReadInputRegisters,
            Self::Coil => FunctionCode::ReadCoils,
            Self::Discrete => FunctionCode::ReadDiscreteInputs,
        }
    }

    /// Get the default write function code for this register type.
    ///
    /// Returns None for read-only register types.
    pub fn write_function_code(self) -> Option<FunctionCode> {
        match self {
            Self::Holding => Some(FunctionCode::WriteMultipleRegisters),
            Self::Coil => Some(FunctionCode::WriteMultipleCoils),
            Self::Input | Self::Discrete => None,
        }
    }
}

/// Bit position within a 16-bit register (0-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BitPosition(u8);

impl BitPosition {
    /// Create a new BitPosition.
    ///
    /// # Panics
    ///
    /// Panics if the position is greater than 15.
    pub fn new(position: u8) -> Self {
        assert!(position < 16, "Bit position must be 0-15");
        Self(position)
    }

    /// Get the bit position as a u8.
    pub fn as_u8(self) -> u8 {
        self.0
    }

    /// Get the bit mask for this position.
    pub fn mask(self) -> u16 {
        1u16 << self.0
    }
}

/// Byte offset within a 16-bit register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOffset {
    /// High byte (bits 8-15)
    High,
    /// Low byte (bits 0-7)
    Low,
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

    #[test]
    fn test_function_code_conversion() {
        assert_eq!(FunctionCode::ReadCoils.as_u8(), 0x01);
        assert_eq!(FunctionCode::ReadHoldingRegisters.as_u8(), 0x03);
        assert_eq!(FunctionCode::WriteMultipleRegisters.as_u8(), 0x10);

        assert_eq!(FunctionCode::from_u8(0x01), Some(FunctionCode::ReadCoils));
        assert_eq!(
            FunctionCode::from_u8(0x03),
            Some(FunctionCode::ReadHoldingRegisters)
        );
        assert_eq!(FunctionCode::from_u8(0xFF), None);
    }

    #[test]
    fn test_function_code_properties() {
        assert!(FunctionCode::ReadCoils.is_read());
        assert!(!FunctionCode::ReadCoils.is_write());
        assert!(FunctionCode::ReadCoils.is_bit_access());
        assert!(!FunctionCode::ReadCoils.is_register_access());

        assert!(FunctionCode::ReadHoldingRegisters.is_read());
        assert!(!FunctionCode::ReadHoldingRegisters.is_write());
        assert!(!FunctionCode::ReadHoldingRegisters.is_bit_access());
        assert!(FunctionCode::ReadHoldingRegisters.is_register_access());

        assert!(!FunctionCode::WriteMultipleRegisters.is_read());
        assert!(FunctionCode::WriteMultipleRegisters.is_write());
        assert!(!FunctionCode::WriteMultipleRegisters.is_bit_access());
        assert!(FunctionCode::WriteMultipleRegisters.is_register_access());

        assert!(FunctionCode::ReadWriteMultipleRegisters.is_read());
        assert!(FunctionCode::ReadWriteMultipleRegisters.is_write());
    }

    #[test]
    fn test_register_type_function_codes() {
        assert_eq!(
            RegisterType::Holding.read_function_code(),
            FunctionCode::ReadHoldingRegisters
        );
        assert_eq!(
            RegisterType::Input.read_function_code(),
            FunctionCode::ReadInputRegisters
        );
        assert_eq!(
            RegisterType::Coil.read_function_code(),
            FunctionCode::ReadCoils
        );

        assert_eq!(
            RegisterType::Holding.write_function_code(),
            Some(FunctionCode::WriteMultipleRegisters)
        );
        assert_eq!(RegisterType::Input.write_function_code(), None);
    }

    #[test]
    fn test_bit_position() {
        let bit0 = BitPosition::new(0);
        assert_eq!(bit0.as_u8(), 0);
        assert_eq!(bit0.mask(), 0x0001);

        let bit7 = BitPosition::new(7);
        assert_eq!(bit7.as_u8(), 7);
        assert_eq!(bit7.mask(), 0x0080);

        let bit15 = BitPosition::new(15);
        assert_eq!(bit15.as_u8(), 15);
        assert_eq!(bit15.mask(), 0x8000);
    }

    #[test]
    #[should_panic(expected = "Bit position must be 0-15")]
    fn test_bit_position_out_of_range() {
        BitPosition::new(16);
    }

    #[test]
    fn test_function_code_names() {
        assert_eq!(FunctionCode::ReadCoils.name(), "Read Coils");
        assert_eq!(
            FunctionCode::ReadHoldingRegisters.name(),
            "Read Holding Registers"
        );
        assert_eq!(
            FunctionCode::WriteMultipleRegisters.name(),
            "Write Multiple Registers"
        );
    }
}
