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

// One request, decoded straight into the struct:
let data = SensorData::read_from_modbus(&mut client).await?;
```

## Core Features

- **Zero runtime overhead** - Serialization code generated at compile time
- **Type-safe** - Catch mapping errors at compile time
- **Minimal** - Thin layer on tokio-modbus
- **Async I/O** - `read_from_modbus` / `write_to_modbus` for whole-struct transfers
- **Efficient packing** - Bit packing (16 bools/register), byte packing (2×u8/register)
- **Validated layout** - Field addresses are checked for contiguity at compile time
- **Per-field endianness** - Big/little word order, overridable per field

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

### Async I/O with tokio-modbus

The `ModbusRead`/`ModbusWrite` extension traits connect the generated mapping to a
live `tokio-modbus` connection. The whole struct is read or written in a single
request at the struct's `base_address`, using the function code implied by its
`register_type`:

```rust
use modbus_mapper::{ModbusMapper, ModbusRead, ModbusWrite};
use tokio_modbus::prelude::*;

let mut ctx = tcp::connect("192.168.1.100:502".parse().unwrap()).await?;

// Read Holding Registers (0x03) at base_address, decode into the struct.
let data = SensorData::read_from_modbus(&mut ctx).await?;

// Encode and Write Multiple Registers (0x10) at base_address.
data.write_to_modbus(&mut ctx).await?;
```

- `holding` → reads with `0x03`, writes with `0x10`.
- `input` → reads with `0x04`; writes return `UnsupportedRegisterType` (read-only).

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
    base_address = 0,              // Absolute wire address of the block, used by I/O (default: 0)
    register_type = "holding",     // "holding" or "input" (default: "holding")
    default_endian = "big"         // "big" or "little" (default: "big")
)]
```

### Field-level

```rust
#[modbus(
    address = 0,                   // Offset within the block; must be contiguous from 0
    endian = "big",                // Override endianness
    bit = 0,                       // Bit position (0-15) for bool
    offset = "high",               // Byte offset ("high"/"low") for u8/i8
    skip                           // Exclude from mapping
)]
```

> **Addressing:** `address` is an *offset within the struct's block*, not a free label.
> The generated buffer is contiguous, so addresses must be gap-free and start at 0
> (packed fields share one address). Gaps/overlaps are a compile error — model a device
> with reserved gaps by splitting it into multiple structs.

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
// ❌ Compile error: non-contiguous layout (gap between 0 and 5)
#[modbus(address = 0)] a: u16,
#[modbus(address = 5)] b: u16,

// ❌ Compile error: bit position out of range (max is 15)
#[modbus(address = 0, bit = 16)] invalid_bit: bool,

// ❌ Compile error: bit attribute on non-bool
#[modbus(address = 0, bit = 0)] not_bool: u8,

// ❌ Compile error: offset attribute on wrong type
#[modbus(address = 0, offset = "high")] not_byte: u16,

// ❌ Compile error: both bit and offset on one field
#[modbus(address = 0, bit = 0, offset = "high")] conflicting: bool,

// ❌ Compile error: register_type = "coil" / "discrete" not supported yet
```

Runtime validation:
```rust
// Register count mismatch
let wrong_size = vec![0u16; 5];
let result = Data::from_registers(&wrong_size);
// Err(ModbusMapperError::RegisterCountMismatch { expected: 3, actual: 5 })
```

## Design Principles

1. **Zero-cost serialization** - Generated code matches hand-written
2. **Type safety first** - Catch errors at compile time
3. **Minimal dependencies** - Thin layer on tokio-modbus
4. **Honest scope** - Reject what isn't modeled yet instead of mis-mapping it
5. **Composable** - Works with existing tokio-modbus code

## Current Status

**Implemented:**
- ✅ Core traits (`ToRegisters`, `FromRegisters`, `ModbusMetadata`)
- ✅ Primitive types (`bool`, `u8`–`u64`, `i8`–`i64`, `f32`, `f64`)
- ✅ Bit packing (up to 16 bools per register) and byte packing (2× u8/i8 per register)
- ✅ Per-field configurable endianness
- ✅ Compile-time contiguous-layout validation
- ✅ Async I/O: `ModbusRead` / `ModbusWrite` over `tokio-modbus` (`holding` / `input`)
- ✅ `FunctionCode` enumeration (19 codes) and `RegisterType` helpers
- ✅ Tested: serialization, packing, contiguity (compile-fail), and mock-device I/O

**Not implemented yet:**
- `String`, `Option<T>`, enums, arrays, tuples, nested structs
- `coil` / `discrete` register types (bit-addressed wire format)
- Server mode (field-level read/write control, change callbacks)

See [TYPE_SPEC.md](TYPE_SPEC.md) for the full breakdown of what is and isn't supported.

## Performance

Serialization is generated at compile time — no reflection, no runtime framework, no
dynamic dispatch. The `to_registers`/`from_registers` code is equivalent to a
hand-written conversion. The async I/O layer adds one `async-trait` box per call, which
is negligible next to the network round-trip it wraps.

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
- 12 core unit tests (endianness, function codes, bit positions)
- 7 primitive type tests (all types, metadata, errors)
- 16 packing tests (bit/byte packing, mixed, roundtrip)
- 5 client I/O tests (mock-device read/write, base-address placement, read-only guard)
- 2 compile-fail tests (non-contiguous layout, unsupported `coil` type)
