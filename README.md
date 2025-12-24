# tokio-modbus-mapper

> Zero-cost procedural macros for type-safe Modbus register mapping in Rust

A compile-time layer over [tokio-modbus](https://github.com/slowtec/tokio-modbus) that generates efficient serialization/deserialization code for Modbus registers.

## Why?

**Working with raw register arrays is error-prone.** This crate lets you work with strongly-typed Rust structs:

```rust
// ❌ Manual register handling
let regs = client.read_holding_registers(0, 4).await?;
let temp = f32::from_bits((regs[0] as u32) << 16 | regs[1] as u32);
let pressure = regs[2];

// ✅ Type-safe struct mapping
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct SensorData {
    #[modbus(address = 0)]
    temperature: f32,      // Registers 0-1
    #[modbus(address = 2)]
    pressure: u16,         // Register 2
}

let registers = client.read_holding_registers(0, 3).await?;
let data = SensorData::from_registers(&registers)?;
```

## Core Features

- **Zero runtime overhead** - All code generated at compile time
- **Type-safe** - Catch mapping errors at compile time
- **Minimal** - Thin layer on tokio-modbus
- **Efficient packing** - Bit packing (16 bools/register), byte packing (2×u8/register)
- **Flexible** - Per-field endianness, multiple register types, custom addresses
- **Complete Modbus support** - All 19 standard function codes

## Quick Start

```toml
[dependencies]
modbus-mapper = "0.1"
tokio-modbus = "0.14"
```

### Basic Usage

```rust
use modbus_mapper::{ModbusMapper, ToRegisters, FromRegisters};

#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct SensorData {
    #[modbus(address = 0)]
    temperature: f32,      // IEEE 754, big-endian, 2 registers

    #[modbus(address = 2)]
    pressure: u16,

    #[modbus(address = 3)]
    humidity: i16,
}

// Serialize to registers
let data = SensorData { temperature: 25.5, pressure: 1013, humidity: 65 };
let registers = data.to_registers();  // Vec<u16>

// Deserialize from registers
let decoded = SensorData::from_registers(&registers)?;
```

## Advanced Features

### Endianness Control

```rust
#[derive(ModbusMapper)]
#[modbus(default_endian = "little")]  // Global default
struct MixedEndian {
    #[modbus(address = 0)]
    value1: u32,                      // Little-endian

    #[modbus(address = 2, endian = "big")]
    value2: u32,                      // Override to big-endian
}
```

### Bit Packing

Pack up to 16 booleans into a single 16-bit register:

```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct StatusFlags {
    #[modbus(address = 0, bit = 0)]
    pump_running: bool,

    #[modbus(address = 0, bit = 1)]
    valve_open: bool,

    #[modbus(address = 0, bit = 2)]
    alarm_active: bool,

    #[modbus(address = 0, bit = 15)]
    system_fault: bool,

    // All 4 booleans stored in register 0
}
```

**Benefits:**
- **Bandwidth**: 16 booleans = 1 register instead of 16
- **Zero-cost**: Compile-time bit manipulation
- **Type-safe**: Bit positions (0-15) validated at compile time

### Byte Packing

Pack two u8/i8 values into high/low bytes of a single register:

```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct BytePacked {
    #[modbus(address = 0, offset = "high")]
    error_code: u8,        // Bits 8-15

    #[modbus(address = 0, offset = "low")]
    status_code: u8,       // Bits 0-7

    #[modbus(address = 1, offset = "high")]
    temperature: i8,       // Signed byte in high position

    #[modbus(address = 1, offset = "low")]
    humidity: i8,          // Signed byte in low position
}
```

### Mixed Packing

Combine normal fields with packed fields:

```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct ControllerState {
    #[modbus(address = 0)]
    setpoint: f32,                    // Registers 0-1: normal field

    #[modbus(address = 2, bit = 0)]
    enable: bool,                     // Register 2, bit 0
    #[modbus(address = 2, bit = 1)]
    auto_mode: bool,                  // Register 2, bit 1
    #[modbus(address = 2, bit = 2)]
    alarm: bool,                      // Register 2, bit 2

    #[modbus(address = 3, offset = "high")]
    mode: u8,                         // Register 3, high byte
    #[modbus(address = 3, offset = "low")]
    state: u8,                        // Register 3, low byte

    #[modbus(address = 4)]
    counter: u32,                     // Registers 4-5: normal field
}
// Total: 6 registers (would be 10 without packing)
```

## Supported Types

| Type | Registers | Endianness | Notes |
|------|-----------|------------|-------|
| `bool` | 1 | - | 0/1 or bit-packed (16/register) |
| `u8`, `i8` | 1 | - | Upper bits zero or byte-packed (2/register) |
| `u16`, `i16` | 1 | - | Native Modbus size |
| `u32`, `i32` | 2 | ✓ | Configurable |
| `u64`, `i64` | 4 | ✓ | Configurable |
| `f32` | 2 | ✓ | IEEE 754 |
| `f64` | 4 | ✓ | IEEE 754 |

## Attributes

### Struct-level

```rust
#[modbus(
    base_address = 0,              // Base address (default: 0)
    register_type = "holding",     // "holding", "input", "coil", "discrete"
    default_endian = "big"         // "big" or "little" (default: "big")
)]
```

### Field-level

```rust
#[modbus(
    address = 0,                   // Register address (required)
    endian = "big",                // Override endianness
    bit = 0,                       // Bit position (0-15) for bool
    offset = "high",               // Byte offset ("high"/"low") for u8/i8
    skip                           // Exclude from mapping
)]
```

## Generated Traits

The `#[derive(ModbusMapper)]` macro generates:

**1. `ToRegisters`** - Serialize struct to registers
```rust
pub trait ToRegisters {
    fn to_registers(&self) -> Vec<u16>;
    fn register_count() -> u16;
}
```

**2. `FromRegisters`** - Deserialize from registers
```rust
pub trait FromRegisters {
    fn from_registers(registers: &[u16]) -> Result<Self>;
}
```

**3. `ModbusMetadata`** - Runtime introspection
```rust
pub trait ModbusMetadata {
    fn base_address() -> u16;
    fn register_type() -> RegisterType;
    fn field_address(field_name: &str) -> Option<u16>;
    fn field_register_count(field_name: &str) -> Option<u16>;
}
```

## Function Code Support

Complete enumeration of 19 standard Modbus function codes:

```rust
use modbus_mapper::FunctionCode;

// Bit access
FunctionCode::ReadCoils                    // 0x01
FunctionCode::ReadDiscreteInputs           // 0x02
FunctionCode::WriteSingleCoil              // 0x05
FunctionCode::WriteMultipleCoils           // 0x0F

// Register access
FunctionCode::ReadHoldingRegisters         // 0x03
FunctionCode::ReadInputRegisters           // 0x04
FunctionCode::WriteSingleRegister          // 0x06
FunctionCode::WriteMultipleRegisters       // 0x10
FunctionCode::ReadWriteMultipleRegisters   // 0x17

// Diagnostics
FunctionCode::ReadExceptionStatus          // 0x07
FunctionCode::Diagnostics                  // 0x08
FunctionCode::GetCommEventCounter          // 0x0B
FunctionCode::GetCommEventLog              // 0x0C

// File record
FunctionCode::ReadFileRecord               // 0x14
FunctionCode::WriteFileRecord              // 0x15

// Advanced
FunctionCode::MaskWriteRegister            // 0x16
FunctionCode::ReadFifoQueue                // 0x18
FunctionCode::ReportServerId               // 0x11
FunctionCode::EncapsulatedInterfaceTransport // 0x2B
```

Helper methods:
```rust
let fc = FunctionCode::ReadHoldingRegisters;
fc.is_read();           // true
fc.is_write();          // false
fc.is_bit_access();     // false
fc.is_register_access(); // true
fc.name();              // "Read Holding Registers"
fc.as_u8();             // 0x03
```

## How It Works

All work happens at **compile time**. The generated code is zero-cost:

```rust
// Your code
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct Data {
    #[modbus(address = 0)]
    temp: f32,
    #[modbus(address = 2, bit = 0)]
    flag: bool,
}

// Generated code (simplified)
impl ToRegisters for Data {
    fn to_registers(&self) -> Vec<u16> {
        let mut registers = Vec::with_capacity(3);
        let bits = self.temp.to_bits();
        registers.push((bits >> 16) as u16);
        registers.push(bits as u16);
        registers.push(if self.flag { 1 } else { 0 });
        registers
    }

    fn register_count() -> u16 { 3 }
}
```

**No runtime overhead.** The generated code is as efficient as hand-written.

## Validation

Compile-time checks prevent common errors:

```rust
// ❌ Compile error: bit position out of range
#[modbus(address = 0, bit = 16)]  // Max is 15
invalid_bit: bool,

// ❌ Compile error: bit attribute on non-bool
#[modbus(address = 0, bit = 0)]
not_bool: u8,

// ❌ Compile error: offset attribute on wrong type
#[modbus(address = 0, offset = "high")]
not_byte: u16,

// ❌ Compile error: both bit and offset
#[modbus(address = 0, bit = 0, offset = "high")]
conflicting: bool,
```

Runtime validation:
```rust
// Register count mismatch
let wrong_size = vec![0u16; 5];
let result = Data::from_registers(&wrong_size);
// Err(ModbusMapperError::RegisterCountMismatch { expected: 3, actual: 5 })
```

## Design Principles

1. **Zero-cost abstraction** - No runtime overhead
2. **Type safety first** - Catch errors at compile time
3. **Minimal dependencies** - Thin layer on tokio-modbus
4. **Industrial-grade** - Complete Modbus protocol support
5. **Composable** - Works with existing tokio-modbus code

## Current Status

**Implemented (Phase 1, 2, 5):**
- ✅ Core traits (ToRegisters, FromRegisters, ModbusMetadata)
- ✅ All primitive types (bool, u8-u64, i8-i64, f32, f64)
- ✅ Bit packing (up to 16 bools per register)
- ✅ Byte packing (2× u8/i8 per register)
- ✅ Configurable endianness (per-field)
- ✅ Complete FunctionCode enumeration (19 codes)
- ✅ RegisterType with function code mappings
- ✅ BitPosition and ByteOffset helper types
- ✅ 35+ comprehensive tests

**Planned:**
- Tokio-modbus integration helpers (async read/write)
- Advanced types (String, Option, enums, arrays)
- Nested struct support
- Server mode (field-level read/write control)

## Performance

**Zero overhead** - identical to hand-written code:

```bash
$ cargo build --release
   Compiling modbus-mapper v0.1.0
   # All code generated at compile time
   # No runtime serialization framework
   # No vtables, no dynamic dispatch
   # Direct register access
```

Generated code compiles to optimal assembly with no indirection.

## Testing

```bash
# All tests
cargo test

# Specific test suite
cargo test --test test_packing

# With output
cargo test -- --nocapture

# Documentation tests
cargo test --doc
```

**Test coverage:**
- 12 core tests (endianness, function codes, bit positions)
- 16 packing tests (bit/byte packing, mixed, roundtrip)
- 7 primitive type tests (all types, metadata, errors)
