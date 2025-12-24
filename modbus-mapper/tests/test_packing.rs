//! Integration tests for bit-packing and byte-packing functionality.

use modbus_mapper::{ModbusMapper, ToRegisters, FromRegisters, ModbusMetadata};

// =============================================================================
// Bit Packing Tests
// =============================================================================

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct BitPackedFlags {
    #[modbus(address = 0, bit = 0)]
    flag0: bool,

    #[modbus(address = 0, bit = 1)]
    flag1: bool,

    #[modbus(address = 0, bit = 2)]
    flag2: bool,

    #[modbus(address = 0, bit = 7)]
    flag7: bool,

    #[modbus(address = 0, bit = 15)]
    flag15: bool,
}

#[test]
fn test_bit_packing_all_false() {
    let flags = BitPackedFlags {
        flag0: false,
        flag1: false,
        flag2: false,
        flag7: false,
        flag15: false,
    };

    let registers = flags.to_registers();
    assert_eq!(registers.len(), 1);
    assert_eq!(registers[0], 0x0000);
}

#[test]
fn test_bit_packing_all_true() {
    let flags = BitPackedFlags {
        flag0: true,
        flag1: true,
        flag2: true,
        flag7: true,
        flag15: true,
    };

    let registers = flags.to_registers();
    assert_eq!(registers.len(), 1);
    // Bits: 0, 1, 2, 7, 15 set
    // Binary: 1000 0000 1000 0111 = 0x8087
    assert_eq!(registers[0], 0x8087);
}

#[test]
fn test_bit_packing_mixed() {
    let flags = BitPackedFlags {
        flag0: true,
        flag1: false,
        flag2: true,
        flag7: false,
        flag15: true,
    };

    let registers = flags.to_registers();
    assert_eq!(registers.len(), 1);
    // Bits: 0, 2, 15 set
    // Binary: 1000 0000 0000 0101 = 0x8005
    assert_eq!(registers[0], 0x8005);
}

#[test]
fn test_bit_packing_roundtrip() {
    let original = BitPackedFlags {
        flag0: true,
        flag1: false,
        flag2: true,
        flag7: true,
        flag15: false,
    };

    let registers = original.to_registers();
    let decoded = BitPackedFlags::from_registers(&registers).expect("Failed to decode");

    assert_eq!(decoded.flag0, original.flag0);
    assert_eq!(decoded.flag1, original.flag1);
    assert_eq!(decoded.flag2, original.flag2);
    assert_eq!(decoded.flag7, original.flag7);
    assert_eq!(decoded.flag15, original.flag15);
}

#[test]
fn test_bit_packing_metadata() {
    assert_eq!(BitPackedFlags::register_count(), 1);
    assert_eq!(BitPackedFlags::field_address("flag0"), Some(0));
    assert_eq!(BitPackedFlags::field_address("flag15"), Some(0));
    assert_eq!(BitPackedFlags::field_register_count("flag0"), Some(1));
}

// =============================================================================
// Byte Packing Tests
// =============================================================================

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct BytePackedData {
    #[modbus(address = 0, offset = "high")]
    high_byte: u8,

    #[modbus(address = 0, offset = "low")]
    low_byte: u8,

    #[modbus(address = 1, offset = "high")]
    signed_high: i8,

    #[modbus(address = 1, offset = "low")]
    signed_low: i8,
}

#[test]
fn test_byte_packing_unsigned() {
    let data = BytePackedData {
        high_byte: 0xAB,
        low_byte: 0xCD,
        signed_high: 0,
        signed_low: 0,
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 2);
    assert_eq!(registers[0], 0xABCD);
}

#[test]
fn test_byte_packing_signed() {
    let data = BytePackedData {
        high_byte: 0,
        low_byte: 0,
        signed_high: -1, // 0xFF
        signed_low: 127, // 0x7F
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 2);
    assert_eq!(registers[1], 0xFF7F);
}

#[test]
fn test_byte_packing_roundtrip() {
    let original = BytePackedData {
        high_byte: 0x12,
        low_byte: 0x34,
        signed_high: -100,
        signed_low: 50,
    };

    let registers = original.to_registers();
    let decoded = BytePackedData::from_registers(&registers).expect("Failed to decode");

    assert_eq!(decoded.high_byte, original.high_byte);
    assert_eq!(decoded.low_byte, original.low_byte);
    assert_eq!(decoded.signed_high, original.signed_high);
    assert_eq!(decoded.signed_low, original.signed_low);
}

#[test]
fn test_byte_packing_metadata() {
    assert_eq!(BytePackedData::register_count(), 2);
    assert_eq!(BytePackedData::field_address("high_byte"), Some(0));
    assert_eq!(BytePackedData::field_address("low_byte"), Some(0));
    assert_eq!(BytePackedData::field_address("signed_high"), Some(1));
    assert_eq!(BytePackedData::field_address("signed_low"), Some(1));
}

// =============================================================================
// Mixed Packing Tests (normal fields + packed fields)
// =============================================================================

#[derive(ModbusMapper)]
#[modbus(base_address = 100, register_type = "holding")]
struct MixedPacking {
    #[modbus(address = 0)]
    temperature: f32, // Registers 0-1

    #[modbus(address = 2, bit = 0)]
    pump_running: bool, // Register 2, bit 0

    #[modbus(address = 2, bit = 1)]
    valve_open: bool, // Register 2, bit 1

    #[modbus(address = 2, bit = 2)]
    alarm_active: bool, // Register 2, bit 2

    #[modbus(address = 3, offset = "high")]
    error_code: u8, // Register 3, high byte

    #[modbus(address = 3, offset = "low")]
    status_code: u8, // Register 3, low byte

    #[modbus(address = 4)]
    counter: u32, // Registers 4-5
}

#[test]
fn test_mixed_packing_to_registers() {
    let data = MixedPacking {
        temperature: 25.5,
        pump_running: true,
        valve_open: false,
        alarm_active: true,
        error_code: 0xE0,
        status_code: 0x0F,
        counter: 12345,
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 6); // f32(2) + bits(1) + bytes(1) + u32(2)

    // Check bit-packed register (register 2)
    assert_eq!(registers[2] & 0x0007, 0x0005); // bits 0,2 set (pump_running, alarm_active)

    // Check byte-packed register (register 3)
    assert_eq!(registers[3], 0xE00F);
}

#[test]
fn test_mixed_packing_roundtrip() {
    let original = MixedPacking {
        temperature: 25.5,
        pump_running: true,
        valve_open: false,
        alarm_active: true,
        error_code: 0xE0,
        status_code: 0x0F,
        counter: 12345,
    };

    let registers = original.to_registers();
    let decoded = MixedPacking::from_registers(&registers).expect("Failed to decode");

    assert_eq!(decoded.temperature, original.temperature);
    assert_eq!(decoded.pump_running, original.pump_running);
    assert_eq!(decoded.valve_open, original.valve_open);
    assert_eq!(decoded.alarm_active, original.alarm_active);
    assert_eq!(decoded.error_code, original.error_code);
    assert_eq!(decoded.status_code, original.status_code);
    assert_eq!(decoded.counter, original.counter);
}

#[test]
fn test_mixed_packing_metadata() {
    assert_eq!(MixedPacking::base_address(), 100);
    assert_eq!(MixedPacking::register_count(), 6);
    assert_eq!(MixedPacking::field_address("temperature"), Some(0));
    assert_eq!(MixedPacking::field_address("pump_running"), Some(2));
    assert_eq!(MixedPacking::field_address("valve_open"), Some(2));
    assert_eq!(MixedPacking::field_address("alarm_active"), Some(2));
    assert_eq!(MixedPacking::field_address("error_code"), Some(3));
    assert_eq!(MixedPacking::field_address("status_code"), Some(3));
    assert_eq!(MixedPacking::field_address("counter"), Some(4));
}

// =============================================================================
// Comprehensive 16-bit Packing Test
// =============================================================================

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct FullBitPacking {
    #[modbus(address = 0, bit = 0)]
    bit0: bool,
    #[modbus(address = 0, bit = 1)]
    bit1: bool,
    #[modbus(address = 0, bit = 2)]
    bit2: bool,
    #[modbus(address = 0, bit = 3)]
    bit3: bool,
    #[modbus(address = 0, bit = 4)]
    bit4: bool,
    #[modbus(address = 0, bit = 5)]
    bit5: bool,
    #[modbus(address = 0, bit = 6)]
    bit6: bool,
    #[modbus(address = 0, bit = 7)]
    bit7: bool,
    #[modbus(address = 0, bit = 8)]
    bit8: bool,
    #[modbus(address = 0, bit = 9)]
    bit9: bool,
    #[modbus(address = 0, bit = 10)]
    bit10: bool,
    #[modbus(address = 0, bit = 11)]
    bit11: bool,
    #[modbus(address = 0, bit = 12)]
    bit12: bool,
    #[modbus(address = 0, bit = 13)]
    bit13: bool,
    #[modbus(address = 0, bit = 14)]
    bit14: bool,
    #[modbus(address = 0, bit = 15)]
    bit15: bool,
}

#[test]
fn test_full_16bit_packing() {
    // Test pattern: alternating bits
    let data = FullBitPacking {
        bit0: true,
        bit1: false,
        bit2: true,
        bit3: false,
        bit4: true,
        bit5: false,
        bit6: true,
        bit7: false,
        bit8: true,
        bit9: false,
        bit10: true,
        bit11: false,
        bit12: true,
        bit13: false,
        bit14: true,
        bit15: false,
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 1);
    // Pattern: 0101 0101 0101 0101 = 0x5555
    assert_eq!(registers[0], 0x5555);

    let decoded = FullBitPacking::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.bit0, true);
    assert_eq!(decoded.bit1, false);
    assert_eq!(decoded.bit15, false);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_single_bit_field() {
    #[derive(ModbusMapper)]
    #[modbus(base_address = 0, register_type = "holding")]
    struct SingleBit {
        #[modbus(address = 0, bit = 5)]
        flag: bool,
    }

    let data = SingleBit { flag: true };
    let registers = data.to_registers();
    assert_eq!(registers.len(), 1);
    assert_eq!(registers[0], 0x0020); // Bit 5 set

    let decoded = SingleBit::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.flag, true);
}

#[test]
fn test_single_byte_high() {
    #[derive(ModbusMapper)]
    #[modbus(base_address = 0, register_type = "holding")]
    struct SingleByteHigh {
        #[modbus(address = 0, offset = "high")]
        value: u8,
    }

    let data = SingleByteHigh { value: 0xAB };
    let registers = data.to_registers();
    assert_eq!(registers.len(), 1);
    assert_eq!(registers[0], 0xAB00);

    let decoded = SingleByteHigh::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.value, 0xAB);
}

#[test]
fn test_single_byte_low() {
    #[derive(ModbusMapper)]
    #[modbus(base_address = 0, register_type = "holding")]
    struct SingleByteLow {
        #[modbus(address = 0, offset = "low")]
        value: u8,
    }

    let data = SingleByteLow { value: 0xCD };
    let registers = data.to_registers();
    assert_eq!(registers.len(), 1);
    assert_eq!(registers[0], 0x00CD);

    let decoded = SingleByteLow::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.value, 0xCD);
}
