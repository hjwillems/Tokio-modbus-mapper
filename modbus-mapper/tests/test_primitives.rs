//! Integration tests for ModbusMapper derive macro with primitive types.

use modbus_mapper::{FromRegisters, ModbusMapper, ModbusMetadata, RegisterType, ToRegisters};

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct SensorData {
    #[modbus(address = 0)]
    temperature: f32,

    #[modbus(address = 2)]
    pressure: u16,

    #[modbus(address = 3)]
    status_flags: u16,
}

#[derive(ModbusMapper)]
#[modbus(base_address = 100, register_type = "input", default_endian = "little")]
struct ComplexData {
    #[modbus(address = 0)]
    count: u32,

    #[modbus(address = 2)]
    value: i32,

    #[modbus(address = 4)]
    timestamp: u64,

    #[modbus(address = 8)]
    enabled: bool,
}

#[test]
fn test_sensor_data_to_registers() {
    let data = SensorData {
        temperature: 25.5,
        pressure: 1013,
        status_flags: 0xFF00,
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 4); // f32 = 2 regs, u16 = 1 reg, u16 = 1 reg
}

#[test]
fn test_sensor_data_from_registers() {
    let data = SensorData {
        temperature: 25.5,
        pressure: 1013,
        status_flags: 0xFF00,
    };

    let registers = data.to_registers();
    let decoded = SensorData::from_registers(&registers).expect("Failed to decode");

    assert_eq!(decoded.temperature, data.temperature);
    assert_eq!(decoded.pressure, data.pressure);
    assert_eq!(decoded.status_flags, data.status_flags);
}

#[test]
fn test_sensor_data_metadata() {
    assert_eq!(SensorData::base_address(), 0);
    assert_eq!(SensorData::register_type(), RegisterType::Holding);
    assert_eq!(SensorData::register_count(), 4);
    assert_eq!(SensorData::field_address("temperature"), Some(0));
    assert_eq!(SensorData::field_address("pressure"), Some(2));
    assert_eq!(SensorData::field_address("status_flags"), Some(3));
    assert_eq!(SensorData::field_address("nonexistent"), None);
}

#[test]
fn test_complex_data_roundtrip() {
    let data = ComplexData {
        count: 12345,
        value: -9876,
        timestamp: 1234567890,
        enabled: true,
    };

    let registers = data.to_registers();
    assert_eq!(registers.len(), 9); // u32 = 2, i32 = 2, u64 = 4, bool = 1

    let decoded = ComplexData::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.count, data.count);
    assert_eq!(decoded.value, data.value);
    assert_eq!(decoded.timestamp, data.timestamp);
    assert_eq!(decoded.enabled, data.enabled);
}

#[test]
fn test_complex_data_metadata() {
    assert_eq!(ComplexData::base_address(), 100);
    assert_eq!(ComplexData::register_type(), RegisterType::Input);
    assert_eq!(ComplexData::field_register_count("count"), Some(2));
    assert_eq!(ComplexData::field_register_count("value"), Some(2));
    assert_eq!(ComplexData::field_register_count("timestamp"), Some(4));
    assert_eq!(ComplexData::field_register_count("enabled"), Some(1));
}

#[test]
fn test_from_registers_count_mismatch() {
    let result = SensorData::from_registers(&[0, 1, 2]); // Wrong count
    assert!(result.is_err());
}

#[test]
fn test_all_primitive_types() {
    #[derive(ModbusMapper)]
    #[modbus(base_address = 0, register_type = "holding")]
    struct AllTypes {
        #[modbus(address = 0)]
        u8_field: u8,

        #[modbus(address = 1)]
        i8_field: i8,

        #[modbus(address = 2)]
        u16_field: u16,

        #[modbus(address = 3)]
        i16_field: i16,

        #[modbus(address = 4)]
        u32_field: u32,

        #[modbus(address = 6)]
        i32_field: i32,

        #[modbus(address = 8)]
        u64_field: u64,

        #[modbus(address = 12)]
        i64_field: i64,

        #[modbus(address = 16)]
        f32_field: f32,

        #[modbus(address = 18)]
        f64_field: f64,

        #[modbus(address = 22)]
        bool_field: bool,
    }

    let data = AllTypes {
        u8_field: 255,
        i8_field: -127,
        u16_field: 65535,
        i16_field: -32767,
        u32_field: 0xDEADBEEF,
        i32_field: -123456,
        u64_field: 0x0123456789ABCDEF,
        i64_field: -987654321,
        f32_field: 1234.5,
        f64_field: 98765.4321,
        bool_field: true,
    };

    // Test serialization
    let registers = data.to_registers();
    assert_eq!(registers.len(), 23); // 1+1+1+1+2+2+4+4+2+4+1

    // Test deserialization
    let decoded = AllTypes::from_registers(&registers).expect("Failed to decode");
    assert_eq!(decoded.u8_field, data.u8_field);
    assert_eq!(decoded.i8_field, data.i8_field);
    assert_eq!(decoded.u16_field, data.u16_field);
    assert_eq!(decoded.i16_field, data.i16_field);
    assert_eq!(decoded.u32_field, data.u32_field);
    assert_eq!(decoded.i32_field, data.i32_field);
    assert_eq!(decoded.u64_field, data.u64_field);
    assert_eq!(decoded.i64_field, data.i64_field);
    assert_eq!(decoded.f32_field, data.f32_field);
    assert_eq!(decoded.f64_field, data.f64_field);
    assert_eq!(decoded.bool_field, data.bool_field);
}
