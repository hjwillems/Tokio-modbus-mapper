//! Endianness conversion utilities for Modbus register types.
//!
//! Modbus uses big-endian byte order within each 16-bit register (network byte order).
//! For multi-register types (u32, u64, f32, f64), the word order can be configured.

/// Endianness configuration for multi-register types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// Big-endian word order (most significant word first).
    /// For f32: [MSW, LSW]
    /// For f64: [W3, W2, W1, W0]
    Big,

    /// Little-endian word order (least significant word first).
    /// For f32: [LSW, MSW]
    /// For f64: [W0, W1, W2, W3]
    Little,
}

impl Default for Endianness {
    fn default() -> Self {
        Self::Big
    }
}

/// Convert u32 to two u16 registers with specified word order.
#[inline]
pub fn u32_to_registers(value: u32, endian: Endianness) -> [u16; 2] {
    let bytes = value.to_be_bytes(); // Each register is big-endian
    let high = u16::from_be_bytes([bytes[0], bytes[1]]);
    let low = u16::from_be_bytes([bytes[2], bytes[3]]);

    match endian {
        Endianness::Big => [high, low],    // MSW first
        Endianness::Little => [low, high], // LSW first
    }
}

/// Convert two u16 registers to u32 with specified word order.
#[inline]
pub fn u32_from_registers(registers: &[u16], endian: Endianness) -> u32 {
    let (high, low) = match endian {
        Endianness::Big => (registers[0], registers[1]),    // MSW first
        Endianness::Little => (registers[1], registers[0]), // LSW first
    };

    let mut bytes = [0u8; 4];
    bytes[0..2].copy_from_slice(&high.to_be_bytes());
    bytes[2..4].copy_from_slice(&low.to_be_bytes());
    u32::from_be_bytes(bytes)
}

/// Convert i32 to two u16 registers with specified word order.
#[inline]
pub fn i32_to_registers(value: i32, endian: Endianness) -> [u16; 2] {
    u32_to_registers(value as u32, endian)
}

/// Convert two u16 registers to i32 with specified word order.
#[inline]
pub fn i32_from_registers(registers: &[u16], endian: Endianness) -> i32 {
    u32_from_registers(registers, endian) as i32
}

/// Convert u64 to four u16 registers with specified word order.
#[inline]
pub fn u64_to_registers(value: u64, endian: Endianness) -> [u16; 4] {
    let bytes = value.to_be_bytes();
    let w3 = u16::from_be_bytes([bytes[0], bytes[1]]);
    let w2 = u16::from_be_bytes([bytes[2], bytes[3]]);
    let w1 = u16::from_be_bytes([bytes[4], bytes[5]]);
    let w0 = u16::from_be_bytes([bytes[6], bytes[7]]);

    match endian {
        Endianness::Big => [w3, w2, w1, w0],    // MSW first
        Endianness::Little => [w0, w1, w2, w3], // LSW first
    }
}

/// Convert four u16 registers to u64 with specified word order.
#[inline]
pub fn u64_from_registers(registers: &[u16], endian: Endianness) -> u64 {
    let (w3, w2, w1, w0) = match endian {
        Endianness::Big => (registers[0], registers[1], registers[2], registers[3]),
        Endianness::Little => (registers[3], registers[2], registers[1], registers[0]),
    };

    let mut bytes = [0u8; 8];
    bytes[0..2].copy_from_slice(&w3.to_be_bytes());
    bytes[2..4].copy_from_slice(&w2.to_be_bytes());
    bytes[4..6].copy_from_slice(&w1.to_be_bytes());
    bytes[6..8].copy_from_slice(&w0.to_be_bytes());
    u64::from_be_bytes(bytes)
}

/// Convert i64 to four u16 registers with specified word order.
#[inline]
pub fn i64_to_registers(value: i64, endian: Endianness) -> [u16; 4] {
    u64_to_registers(value as u64, endian)
}

/// Convert four u16 registers to i64 with specified word order.
#[inline]
pub fn i64_from_registers(registers: &[u16], endian: Endianness) -> i64 {
    u64_from_registers(registers, endian) as i64
}

/// Convert f32 to two u16 registers with specified word order.
#[inline]
pub fn f32_to_registers(value: f32, endian: Endianness) -> [u16; 2] {
    u32_to_registers(value.to_bits(), endian)
}

/// Convert two u16 registers to f32 with specified word order.
#[inline]
pub fn f32_from_registers(registers: &[u16], endian: Endianness) -> f32 {
    f32::from_bits(u32_from_registers(registers, endian))
}

/// Convert f64 to four u16 registers with specified word order.
#[inline]
pub fn f64_to_registers(value: f64, endian: Endianness) -> [u16; 4] {
    u64_to_registers(value.to_bits(), endian)
}

/// Convert four u16 registers to f64 with specified word order.
#[inline]
pub fn f64_from_registers(registers: &[u16], endian: Endianness) -> f64 {
    f64::from_bits(u64_from_registers(registers, endian))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_big_endian() {
        let value: u32 = 0x12345678;
        let regs = u32_to_registers(value, Endianness::Big);
        assert_eq!(regs, [0x1234, 0x5678]);
        assert_eq!(u32_from_registers(&regs, Endianness::Big), value);
    }

    #[test]
    fn test_u32_little_endian() {
        let value: u32 = 0x12345678;
        let regs = u32_to_registers(value, Endianness::Little);
        assert_eq!(regs, [0x5678, 0x1234]);
        assert_eq!(u32_from_registers(&regs, Endianness::Little), value);
    }

    #[test]
    fn test_f32_conversion() {
        let value: f32 = 123.456;
        let regs_big = f32_to_registers(value, Endianness::Big);
        let regs_little = f32_to_registers(value, Endianness::Little);

        assert_eq!(f32_from_registers(&regs_big, Endianness::Big), value);
        assert_eq!(f32_from_registers(&regs_little, Endianness::Little), value);
        assert_ne!(regs_big, regs_little);
    }

    #[test]
    fn test_f64_conversion() {
        let value: f64 = 123.456789;
        let regs_big = f64_to_registers(value, Endianness::Big);
        let regs_little = f64_to_registers(value, Endianness::Little);

        assert_eq!(f64_from_registers(&regs_big, Endianness::Big), value);
        assert_eq!(f64_from_registers(&regs_little, Endianness::Little), value);
        assert_ne!(regs_big, regs_little);
    }

    #[test]
    fn test_i32_negative() {
        let value: i32 = -12345;
        let regs = i32_to_registers(value, Endianness::Big);
        assert_eq!(i32_from_registers(&regs, Endianness::Big), value);
    }
}
