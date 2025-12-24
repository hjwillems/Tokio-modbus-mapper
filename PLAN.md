# Tokio Modbus Mapper - Implementation Plan

## 1. Project Overview

A Rust procedural macro crate that automatically maps Rust struct fields to Modbus registers with **comprehensive type support** for both **client and server** use cases:

### Supported Types (v1.0):
- **Primitives**: u8-u64, i8-i64, f32, f64, bool
- **Strings**: Fixed-length String with configurable encoding/padding
- **Arrays**: Fixed-size arrays up to 3D
- **Option<T>**: Nullable types with sentinel values or validity flags
- **Enums**: #[repr] enums with discriminant validation
- **Bit fields**: Pack multiple bools/integers into single registers
- **Nested structs**: Composition via #[modbus(flatten)]
- **Tuples**: Up to 8-element tuples

### Key Features:
- Configurable endianness (big/little) per field
- Compile-time validation of all mappings
- Type-safe serialization/deserialization
- Both client (master) and server (slave) modes
- Server features: readonly/writeonly fields, validation, change callbacks
- Support for all Modbus register types (Holding, Input, Coils, Discrete)
- Field-level and struct-level read/write operations
- Thread-safe server patterns

## 2. Design Philosophy

### Thin, Zero-Cost Abstraction Layer

This crate is designed as an **extremely lightweight layer** on top of `tokio-modbus`:

**Core Principles**:
- ✅ **Zero runtime overhead** - All work done at compile time via proc macros
- ✅ **No runtime dependencies** except `tokio-modbus` (and `thiserror` for errors)
- ✅ **Generated code only** - No complex runtime logic
- ✅ **Compile-time validation** - Catch errors before running
- ✅ **No heap allocations** in hot path (register conversions use stack)
- ✅ **Inline-friendly** - All generated methods can be inlined
- ✅ **Minimal binary size impact** - Only pay for what you use

**What this is NOT**:
- ❌ Not a framework - just a derive macro
- ❌ Not a Modbus implementation - delegates to `tokio-modbus`
- ❌ Not a runtime - pure compile-time code generation
- ❌ Not opinionated - you control the mapping

**Size Budget**:
- Proc macro crate: Can be large (only used during compilation)
- Runtime library: < 50 KB compiled
- Generated code: Minimal, inlineable functions

**Philosophy**:
> "The best abstraction is one you don't pay for. Generate perfect code at compile time, run it with zero overhead at runtime."

## 3. Crate Structure

```
tokio-modbus-mapper/
├── Cargo.toml                    # Workspace definition
├── modbus-mapper/                # Main library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Re-exports macro + runtime types
│       ├── types.rs              # Trait definitions for serialization
│       ├── endian.rs             # Endianness handling
│       └── error.rs              # Error types
└── modbus-mapper-derive/         # Procedural macro crate
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Proc macro entry point
        ├── parse.rs              # Parse struct attributes
        ├── codegen.rs            # Code generation logic
        └── register_allocator.rs # Register address allocation
```

## 3. Modbus Protocol Fundamentals

### Register Types:
1. **Holding Registers** (Read/Write, 16-bit, typically 40001-49999)
2. **Input Registers** (Read-only, 16-bit, typically 30001-39999)
3. **Coils** (Read/Write, 1-bit, typically 00001-09999)
4. **Discrete Inputs** (Read-only, 1-bit, typically 10001-19999)

### Register Size:
- Each register = 16 bits (2 bytes)
- Multi-register types span consecutive addresses

## 4. Type Mappings

### Primitive Types

| Rust Type | Registers | Bits | Default Endian | Notes |
|-----------|-----------|------|----------------|-------|
| bool      | 1 coil or bit | 1 | N/A | Coil or bit field in register |
| u8        | 1         | 16*  | Big | Stored in lower byte |
| i8        | 1         | 16*  | Big | Stored in lower byte |
| u16       | 1         | 16   | Big | |
| i16       | 1         | 16   | Big | |
| u32       | 2         | 32   | Big | Configurable word order |
| i32       | 2         | 32   | Big | Configurable word order |
| u64       | 4         | 64   | Big | Configurable word order |
| i64       | 4         | 64   | Big | Configurable word order |
| f32       | 2         | 32   | Big | IEEE 754, configurable word order |
| f64       | 4         | 64   | Big | IEEE 754, configurable word order |

*Note: u8/i8 stored in lower byte of 16-bit register

### Complex Types

| Rust Type | Registers | Configuration | Notes |
|-----------|-----------|---------------|-------|
| String | N (specified) | `length = N` | Fixed-length, 2 bytes per register |
| [T; N] | N × size(T) | Compile-time length | Sequential layout |
| [[T; Y]; X] | X × Y × size(T) | Up to 3D | Row-major layout |
| Option<T> | size(T) or size(T)+1 | `none_value` or `none_flag` | Sentinel or validity flag |
| Enum | 1-4 | `#[repr(u8/u16/u32/u64)]` | Discriminant validation |
| (T1, T2, ...) | Σ size(Ti) | Up to 8 elements | Sequential layout |
| Nested Struct | sum of fields | `#[modbus(flatten)]` | Inline fields |
| Bit field | Shared register | `bit = N` or `bits = N..M` | Pack multiple into one register |

## 5. Macro API Design

### Basic Usage Example:

```rust
use modbus_mapper::ModbusMapper;

#[derive(ModbusMapper)]
#[modbus(base_address = 0)]
struct SensorData {
    #[modbus(address = 0, endian = "big")]
    temperature: f32,           // Registers 0-1

    #[modbus(address = 2, endian = "little")]
    pressure: f32,              // Registers 2-3

    #[modbus(address = 4)]
    humidity: u16,              // Register 4

    #[modbus(address = 5)]
    status_flags: u32,          // Registers 5-6

    #[modbus(address = 7, count = 10)]
    samples: [u16; 10],         // Registers 7-16
}
```

### Attribute Options:

#### Struct-level attributes:
```rust
#[modbus(
    base_address = <u16>,           // Optional: base offset for all fields
    register_type = "holding",      // "holding", "input", "coil", "discrete"
    default_endian = "big"          // "big" or "little"
)]
```

#### Field-level attributes:
```rust
#[modbus(
    address = <u16>,                // Required: register offset
    endian = "big"|"little",        // Optional: override default
    skip                            // Optional: skip this field
)]
```

### Advanced Configuration:

```rust
#[derive(ModbusMapper)]
#[modbus(
    base_address = 1000,
    register_type = "holding",
    default_endian = "big"
)]
struct IndustrialController {
    #[modbus(address = 0, endian = "little")]
    setpoint: f64,              // Modbus addresses 1000-1003

    #[modbus(address = 4)]
    current_value: f64,         // Addresses 1004-1007

    #[modbus(skip)]
    internal_state: String,     // Not mapped to Modbus

    #[modbus(address = 8, endian = "big")]
    pid_gains: [f32; 3],        // Addresses 1008-1013 (Kp, Ki, Kd)
}
```

## 6. Generated Code

The macro will generate:

### 1. Register Mapping Methods:
```rust
impl SensorData {
    /// Returns the total number of registers required
    pub fn register_count() -> u16;

    /// Returns the starting address
    pub fn base_address() -> u16;

    /// Get address for a specific field
    pub fn field_address(field: &str) -> Option<u16>;
}
```

### 2. Serialization to Modbus Registers:
```rust
impl SensorData {
    /// Serialize entire struct to holding registers
    pub fn to_registers(&self) -> Vec<u16>;

    /// Serialize a specific field to registers
    pub fn field_to_registers(&self, field: &str) -> Option<Vec<u16>>;
}
```

### 3. Deserialization from Modbus Registers:
```rust
impl SensorData {
    /// Deserialize from holding registers
    pub fn from_registers(registers: &[u16]) -> Result<Self, ModbusMapperError>;

    /// Update a specific field from registers
    pub fn update_field_from_registers(
        &mut self,
        field: &str,
        registers: &[u16]
    ) -> Result<(), ModbusMapperError>;
}
```

### 4. Direct Modbus I/O (with tokio-modbus integration):
```rust
impl SensorData {
    /// Read entire struct from Modbus device
    pub async fn read_from_modbus(
        ctx: &mut tokio_modbus::client::Context
    ) -> Result<Self, ModbusMapperError>;

    /// Write entire struct to Modbus device
    pub async fn write_to_modbus(
        &self,
        ctx: &mut tokio_modbus::client::Context
    ) -> Result<(), ModbusMapperError>;

    /// Read a specific field from Modbus
    pub async fn read_field_from_modbus(
        &mut self,
        ctx: &mut tokio_modbus::client::Context,
        field: &str
    ) -> Result<(), ModbusMapperError>;

    /// Write a specific field to Modbus
    pub async fn write_field_to_modbus(
        &self,
        ctx: &mut tokio_modbus::client::Context,
        field: &str
    ) -> Result<(), ModbusMapperError>;
}
```

## 7. Endianness Handling

### Word Order (for multi-register types):
- **Big-endian (Motorola)**: Most significant word first
  - f32 = [HIGH_WORD, LOW_WORD]
  - f64 = [WORD_3, WORD_2, WORD_1, WORD_0]

- **Little-endian (Intel)**: Least significant word first
  - f32 = [LOW_WORD, HIGH_WORD]
  - f64 = [WORD_0, WORD_1, WORD_2, WORD_3]

### Byte Order (within each 16-bit register):
- Modbus standard: **Big-endian** (network byte order)
- Always MSB first in each register

### Configuration Options:
```rust
pub enum Endianness {
    Big,        // Big-endian word order
    Little,     // Little-endian word order
}
```

## 8. Industry-Specific Features

### 1. Common Industrial Formats:
```rust
// IEEE 754 floating point with configurable word order
#[modbus(address = 0, endian = "big", format = "ieee754")]
flow_rate: f32,

// Scaled integers (e.g., value × 100 for 2 decimal places)
#[modbus(address = 2, scale = 100)]
temperature_scaled: i16,  // Actual value = register_value / 100.0
```

### 2. Bit Field Support (future):
```rust
#[modbus(address = 0, bit = 0)]
alarm_active: bool,

#[modbus(address = 0, bit = 1)]
motor_running: bool,
```

### 3. String Support (future):
```rust
#[modbus(address = 10, length = 16)]  // 16 registers = 32 chars
device_name: String,
```

## 9. Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModbusMapperError {
    #[error("Register count mismatch: expected {expected}, got {actual}")]
    RegisterCountMismatch { expected: usize, actual: usize },

    #[error("Invalid field name: {0}")]
    InvalidField(String),

    #[error("Endianness conversion error")]
    EndiannessError,

    #[error("Modbus I/O error: {0}")]
    ModbusError(#[from] tokio_modbus::Error),

    #[error("Address out of range: {0}")]
    AddressOutOfRange(u16),
}
```

## 10. Dependencies

### Runtime Dependencies (Minimal!)

The runtime crate has **only essential dependencies**:

```toml
# modbus-mapper/Cargo.toml
[dependencies]
# Core Modbus functionality - our only heavy dependency
tokio-modbus = "0.14"

# Only needed for async traits in generated code (re-export from tokio-modbus)
tokio = { version = "1", default-features = false }

# Lightweight error handling
thiserror = "1"

# Optional: Only if we need explicit byte order control
# Most conversions can use native endianness functions
byteorder = { version = "1", optional = true }
```

**Size impact**: ~200 KB (mostly tokio-modbus)

### Compile-Time Dependencies (Proc Macro)

The proc macro crate can be heavier since it's only used during compilation:

```toml
# modbus-mapper-derive/Cargo.toml
[dependencies]
# Parsing Rust syntax
syn = { version = "2", features = ["full", "extra-traits"] }

# Code generation
quote = "1"
proc-macro2 = "1"

# Attribute parsing helper (much cleaner than manual parsing)
darling = "0.20"
```

**Note**: Proc macro dependencies don't affect your final binary size!

## 11. Implementation Phases

### Phase 1: Core Infrastructure
- [ ] Set up workspace with two crates (modbus-mapper + modbus-mapper-derive)
- [ ] Define basic traits and types
- [ ] Implement endianness conversion utilities (big/little word order)
- [ ] Create comprehensive error types

### Phase 2: Primitive Types - Proc Macro
- [ ] Parse `#[derive(ModbusMapper)]` and `#[derive(ModbusEnum)]`
- [ ] Parse struct-level attributes (base_address, register_type, default_endian)
- [ ] Parse field-level attributes (address, endian, skip)
- [ ] Validate attribute combinations at compile time
- [ ] Generate code for primitive types (u8-u64, i8-i64, f32, f64, bool)

### Phase 3: Basic Serialization
- [ ] Generate `to_registers()` for primitive types
- [ ] Generate `from_registers()` for primitive types
- [ ] Handle endianness for multi-register types (u32, u64, f32, f64)
- [ ] Support for fixed-size arrays `[T; N]`
- [ ] Generate metadata methods (register_count, base_address, field_address)

### Phase 4: Tokio-Modbus Client Integration
- [ ] Generate async `read_from_modbus()` method
- [ ] Generate async `write_to_modbus()` method
- [ ] Generate async `read_field_from_modbus()` method
- [ ] Generate async `write_field_to_modbus()` method
- [ ] Handle different register types (holding/input/coils/discrete)

### Phase 5: Advanced Types - Strings & Option
- [ ] Fixed-length String support with `length` attribute
- [ ] String encoding (ASCII, UTF-8) and padding (null, space, none)
- [ ] Option<T> with sentinel value strategy (`none_value`)
- [ ] Option<T> with validity flag strategy (`none_flag`)
- [ ] NaN handling for Option<f32>/Option<f64>

### Phase 6: Advanced Types - Enums & Bit Fields
- [ ] Enum support with `#[repr(u8/u16/u32/u64)]`
- [ ] Discriminant validation on read
- [ ] Invalid enum error handling
- [ ] Bit field support (`bit = N`)
- [ ] Bit range support (`bits = N..M`)
- [ ] Overlap detection for bit fields

### Phase 7: Advanced Types - Nested & Tuples
- [ ] Nested struct support with `flatten` attribute
- [ ] Multi-level nesting support
- [ ] Tuple support up to 8 elements
- [ ] Multi-dimensional arrays (2D, 3D)
- [ ] Row-major layout for multi-dim arrays

### Phase 8: Server Mode Support
- [ ] `update_from_registers()` method for write handling
- [ ] `update_field_from_registers()` for field writes
- [ ] Read-only field support (`readonly` attribute)
- [ ] Write-only field support (`writeonly` attribute)
- [ ] Validation support (`validate` range, `validate_with` function)
- [ ] Change callback support (`on_change` attribute)

### Phase 9: Testing & Validation
- [ ] Unit tests for all type conversions
- [ ] Unit tests for endianness handling
- [ ] Enum discriminant validation tests
- [ ] Bit field packing/unpacking tests
- [ ] Integration tests with tokio-modbus
- [ ] Integration tests with Modbus simulator
- [ ] Server mode integration tests
- [ ] Property-based testing with proptest

### Phase 10: Documentation & Examples
- [ ] API documentation for all public items
- [ ] Type specification document (TYPE_SPEC.md) ✓
- [ ] Implementation plan (PLAN.md) ✓
- [ ] Example: Simple sensor client
- [ ] Example: Complex device with all types
- [ ] Example: Modbus server implementation
- [ ] Example: Thread-safe server with Arc<RwLock>
- [ ] Tutorial documentation

### Phase 11: Performance & Polish
- [ ] Benchmark register conversion performance
- [ ] Benchmark serialization/deserialization
- [ ] Optimize generated code size
- [ ] Comprehensive compile-time error messages
- [ ] Helpful error messages from macro
- [ ] Clippy lints and fixes
- [ ] CI/CD pipeline setup
- [ ] Crates.io release preparation

## 12. Testing Strategy

### Unit Tests:
```rust
#[test]
fn test_f32_big_endian_conversion() {
    let value: f32 = 123.456;
    let registers = f32_to_registers_be(value);
    let decoded = f32_from_registers_be(&registers);
    assert_eq!(value, decoded);
}
```

### Integration Tests:
```rust
#[tokio::test]
async fn test_read_write_struct() {
    let mut ctx = create_test_context().await;
    let data = SensorData { ... };

    data.write_to_modbus(&mut ctx).await.unwrap();
    let read_data = SensorData::read_from_modbus(&mut ctx).await.unwrap();

    assert_eq!(data, read_data);
}
```

### Modbus Simulator:
- Use `tokio-modbus` with test server
- Or use external simulator like `pymodbus` or `modbus-tk`

## 13. Example Use Cases

### Industrial PLC Communication:
```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct PLCData {
    #[modbus(address = 0)]
    speed_setpoint: u16,

    #[modbus(address = 1, endian = "little")]
    actual_position: f32,

    #[modbus(address = 3)]
    error_code: u16,
}

async fn control_loop(ctx: &mut Context) -> Result<()> {
    let mut plc = PLCData::read_from_modbus(ctx).await?;

    plc.speed_setpoint = calculate_speed(&plc);
    plc.write_field_to_modbus(ctx, "speed_setpoint").await?;

    Ok(())
}
```

### Energy Meter Reading:
```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "input", default_endian = "big")]
struct EnergyMeter {
    #[modbus(address = 0)]
    voltage: f32,

    #[modbus(address = 2)]
    current: f32,

    #[modbus(address = 4)]
    power: f32,

    #[modbus(address = 6)]
    energy: f64,
}
```

## 14. Open Questions & Design Decisions

1. **Default Addressing**: Auto-calculate addresses vs. require explicit?
   - **Decision**: Require explicit for clarity and control

2. **Register Type Mixing**: Allow different register types in one struct?
   - **Decision**: Single register type per struct for simplicity

3. **Async Runtime**: Require tokio or support other runtimes?
   - **Decision**: Focus on tokio initially, extensible design

4. **Partial Updates**: Support reading/writing individual fields?
   - **Decision**: Yes, via `field_to_registers()` and field-specific methods

5. **Validation**: Compile-time vs runtime validation?
   - **Decision**: Maximum compile-time validation via proc macro

## 15. Future Enhancements

- [ ] Support for custom types via `ModbusSerialize` trait
- [ ] Automatic address allocation (sequential)
- [ ] Struct composition (nested structs)
- [ ] Conditional compilation based on Modbus variant (RTU/TCP)
- [ ] Code generation for multiple languages (C header files, etc.)
- [ ] Interactive mapper UI/tool
- [ ] Hot-reload configuration
- [ ] OPC UA bridge

---

## Next Steps

1. Review and approve this plan
2. Set up the workspace structure
3. Start with Phase 1: Core Infrastructure
4. Iterate based on feedback and real-world usage
