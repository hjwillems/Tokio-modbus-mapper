# Tokio Modbus Mapper - Implementation Plan

## 1. Project Overview

A Rust procedural macro crate that automatically maps Rust struct fields to Modbus registers, supporting:
- Multiple Rust primitive types (u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, bool)
- Configurable endianness (Big-endian, Little-endian)
- Industry-standard Modbus configurations
- Automatic register allocation
- Type-safe read/write operations
- Support for all Modbus register types (Holding, Input, Coils, Discrete Inputs)

## 2. Crate Structure

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

| Rust Type | Registers | Bits | Default Endian |
|-----------|-----------|------|----------------|
| bool      | 1 coil    | 1    | N/A            |
| u8        | 1         | 16*  | Big            |
| i8        | 1         | 16*  | Big            |
| u16       | 1         | 16   | Big            |
| i16       | 1         | 16   | Big            |
| u32       | 2         | 32   | Big            |
| i32       | 2         | 32   | Big            |
| u64       | 4         | 64   | Big            |
| i64       | 4         | 64   | Big            |
| f32       | 2         | 32   | Big            |
| f64       | 4         | 64   | Big            |

*Note: u8/i8 stored in lower byte of 16-bit register

### Array Support:
- Fixed-size arrays: `[T; N]` where T is any supported type
- Register allocation: `N × registers_per_T`

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

```toml
# modbus-mapper/Cargo.toml
[dependencies]
tokio-modbus = "0.14"
tokio = { version = "1", features = ["full"] }
thiserror = "1"
byteorder = "1"

# modbus-mapper-derive/Cargo.toml
[dependencies]
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
darling = "0.20"  # For parsing attributes easily
```

## 11. Implementation Phases

### Phase 1: Core Infrastructure ✓
- [ ] Set up workspace with two crates
- [ ] Define basic traits and types
- [ ] Implement endianness conversion utilities
- [ ] Create error types

### Phase 2: Basic Proc Macro ✓
- [ ] Parse `#[derive(ModbusMapper)]`
- [ ] Parse struct-level attributes
- [ ] Parse field-level attributes
- [ ] Validate attribute combinations

### Phase 3: Code Generation - Register Serialization ✓
- [ ] Generate `to_registers()` for primitive types
- [ ] Generate `from_registers()` for primitive types
- [ ] Handle endianness for multi-register types
- [ ] Support for arrays

### Phase 4: Tokio-Modbus Integration ✓
- [ ] Generate async read methods
- [ ] Generate async write methods
- [ ] Handle different register types (holding/input/coils)
- [ ] Implement field-specific I/O

### Phase 5: Advanced Features ✓
- [ ] Bit field support
- [ ] String support
- [ ] Scaled integers
- [ ] Custom type support via traits

### Phase 6: Testing & Documentation ✓
- [ ] Unit tests for type conversions
- [ ] Integration tests with Modbus simulator
- [ ] Documentation with examples
- [ ] Performance benchmarks

### Phase 7: Polish ✓
- [ ] Comprehensive error messages from macro
- [ ] Compile-time validation
- [ ] Examples for common use cases
- [ ] CI/CD setup

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
