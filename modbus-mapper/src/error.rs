//! Error types for Modbus mapper operations.

use std::fmt;

/// Errors that can occur during Modbus mapping operations.
#[derive(Debug, thiserror::Error)]
pub enum ModbusMapperError {
    /// Register count mismatch between expected and actual.
    #[error("Register count mismatch: expected {expected}, got {actual}")]
    RegisterCountMismatch {
        /// Expected number of registers
        expected: usize,
        /// Actual number of registers received
        actual: usize,
    },

    /// Invalid field name requested.
    #[error("Invalid field name: '{0}'")]
    InvalidField(String),

    /// Endianness conversion error.
    #[error("Endianness conversion error")]
    EndiannessError,

    /// Modbus I/O error from tokio-modbus.
    #[error("Modbus I/O error: {0}")]
    ModbusError(#[from] tokio_modbus::Error),

    /// Address out of valid Modbus range.
    #[error("Address out of range: {0}")]
    AddressOutOfRange(u16),

    /// Invalid enum discriminant read from registers.
    #[error("Invalid enum discriminant: {value} for type '{type_name}'")]
    InvalidEnum {
        /// The invalid discriminant value
        value: u64,
        /// Name of the enum type
        type_name: &'static str,
    },

    /// String encoding/decoding error.
    #[error("String conversion error: {0}")]
    StringError(String),

    /// Validation error for field value.
    #[error("Validation failed for field '{field}': {message}")]
    ValidationError {
        /// Field name that failed validation
        field: &'static str,
        /// Validation error message
        message: String,
    },

    /// Bit field overlap detected.
    #[error("Bit field overlap at register {address}, bits {bit1} and {bit2}")]
    BitFieldOverlap {
        /// Register address where overlap occurred
        address: u16,
        /// First overlapping bit
        bit1: u8,
        /// Second overlapping bit
        bit2: u8,
    },

    /// Register type mismatch (e.g., trying to use coil operation on holding register).
    #[error("Register type mismatch: expected {expected}, operation requires {required}")]
    RegisterTypeMismatch {
        /// Expected register type
        expected: &'static str,
        /// Required register type for operation
        required: &'static str,
    },

    /// UTF-8 decoding error when reading string from registers.
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Custom error for user-defined validation functions.
    #[error("Custom validation error: {0}")]
    Custom(String),
}

/// Result type for Modbus mapper operations.
pub type Result<T> = std::result::Result<T, ModbusMapperError>;

impl ModbusMapperError {
    /// Create a validation error.
    pub fn validation(field: &'static str, message: impl fmt::Display) -> Self {
        Self::ValidationError {
            field,
            message: message.to_string(),
        }
    }

    /// Create a custom error.
    pub fn custom(message: impl fmt::Display) -> Self {
        Self::Custom(message.to_string())
    }

    /// Create an invalid enum error.
    pub fn invalid_enum(value: u64, type_name: &'static str) -> Self {
        Self::InvalidEnum { value, type_name }
    }

    /// Create a string error.
    pub fn string_error(message: impl fmt::Display) -> Self {
        Self::StringError(message.to_string())
    }
}
