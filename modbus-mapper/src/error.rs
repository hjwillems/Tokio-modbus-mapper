//! Error types for Modbus mapper operations.
//!
//! The error surface is intentionally small: it only covers conditions that the
//! currently-implemented code paths can actually produce. New variants are added
//! alongside the features that need them, not ahead of them.

use crate::RegisterType;

/// Errors that can occur during Modbus mapping operations.
#[derive(Debug, thiserror::Error)]
pub enum ModbusMapperError {
    /// The number of registers provided to [`from_registers`](crate::FromRegisters::from_registers)
    /// does not match the number the type expects.
    #[error("register count mismatch: expected {expected}, got {actual}")]
    RegisterCountMismatch {
        /// Number of registers the type expects.
        expected: usize,
        /// Number of registers actually provided.
        actual: usize,
    },

    /// Transport or protocol error surfaced by `tokio-modbus`.
    #[error("modbus transport error: {0}")]
    Transport(#[from] tokio_modbus::Error),

    /// The Modbus server responded with an exception (e.g. illegal data address).
    #[error("modbus exception response: {0:?}")]
    Exception(tokio_modbus::Exception),

    /// An I/O operation was requested for a register type that does not support it.
    ///
    /// For example, writing to an `input` register block, or reading/writing a
    /// `coil`/`discrete` block (which the `Vec<u16>` register model does not model yet).
    #[error("register type '{register_type:?}' does not support operation '{operation}'")]
    UnsupportedRegisterType {
        /// The register type the operation was attempted on.
        register_type: RegisterType,
        /// The operation that is not supported (`"read"` or `"write"`).
        operation: &'static str,
    },
}

impl From<tokio_modbus::Exception> for ModbusMapperError {
    fn from(exception: tokio_modbus::Exception) -> Self {
        Self::Exception(exception)
    }
}

/// Result type for Modbus mapper operations.
pub type Result<T> = std::result::Result<T, ModbusMapperError>;
