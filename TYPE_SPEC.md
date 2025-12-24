# Type Specification - What Can and Cannot Be Mapped

## Design Principle

**Modbus Constraint**: Modbus operates with fixed-size, contiguous register blocks. Every type must have:
1. **Compile-time known size** - We must know exactly how many registers it needs
2. **Deterministic layout** - Same data always maps to same registers
3. **No runtime indirection** - Direct memory-to-register mapping

---

## ✅ SUPPORTED TYPES (Version 1.0)

### 1. Unsigned Integers

| Type | Size | Registers | Layout |
|------|------|-----------|--------|
| `u8` | 8-bit | 1 | Stored in lower byte of register |
| `u16` | 16-bit | 1 | Direct mapping to single register |
| `u32` | 32-bit | 2 | Two consecutive registers |
| `u64` | 64-bit | 4 | Four consecutive registers |

**Example**:
```rust
#[derive(ModbusMapper)]
struct Example {
    #[modbus(address = 0)]
    byte_value: u8,      // Register 0: 0x00XX (upper byte unused)

    #[modbus(address = 1)]
    word_value: u16,     // Register 1: 0xXXXX

    #[modbus(address = 2)]
    dword_value: u32,    // Registers 2-3

    #[modbus(address = 4)]
    qword_value: u64,    // Registers 4-7
}
```

**Register count**: 8 total

---

### 2. Signed Integers

| Type | Size | Registers | Encoding |
|------|------|-----------|----------|
| `i8` | 8-bit | 1 | Two's complement in lower byte |
| `i16` | 16-bit | 1 | Two's complement |
| `i32` | 32-bit | 2 | Two's complement |
| `i64` | 64-bit | 4 | Two's complement |

**Example**:
```rust
#[derive(ModbusMapper)]
struct Sensors {
    #[modbus(address = 0)]
    temperature: i16,    // -32768 to +32767

    #[modbus(address = 1)]
    pressure: i32,       // Registers 1-2
}
```

---

### 3. Floating Point

| Type | Size | Registers | Standard |
|------|------|-----------|----------|
| `f32` | 32-bit | 2 | IEEE 754 single precision |
| `f64` | 64-bit | 4 | IEEE 754 double precision |

**Endianness matters!** Word order can be configured:
- **Big-endian**: `[MSW, LSW]` for f32, `[W3, W2, W1, W0]` for f64
- **Little-endian**: `[LSW, MSW]` for f32, `[W0, W1, W2, W3]` for f64

**Example**:
```rust
#[derive(ModbusMapper)]
#[modbus(default_endian = "big")]
struct FloatData {
    #[modbus(address = 0, endian = "big")]
    flow_rate: f32,          // IEEE 754, big-endian word order

    #[modbus(address = 2, endian = "little")]
    temperature: f32,        // IEEE 754, little-endian word order

    #[modbus(address = 4)]
    precise_value: f64,      // Registers 4-7
}
```

---

### 4. Boolean (Coils)

| Type | Size | Registers | Register Type |
|------|------|-----------|---------------|
| `bool` | 1-bit | 1 coil | Coil or Discrete Input |

**Important**:
- `bool` fields use **Coils** (read/write) or **Discrete Inputs** (read-only)
- Cannot mix `bool` with numeric types in the same struct (different register types)
- One `bool` = one coil address

**Example**:
```rust
// VALID: All boolean fields
#[derive(ModbusMapper)]
#[modbus(register_type = "coil", base_address = 0)]
struct StatusBits {
    #[modbus(address = 0)]
    motor_running: bool,     // Coil 0

    #[modbus(address = 1)]
    alarm_active: bool,      // Coil 1

    #[modbus(address = 2)]
    door_open: bool,         // Coil 2
}

// INVALID: Cannot mix bool with numeric types
#[derive(ModbusMapper)]
struct Invalid {
    #[modbus(address = 0)]
    temperature: u16,        // Holding register

    #[modbus(address = 1)]
    alarm: bool,             // ❌ ERROR: Different register type!
}
```

---

### 5. Fixed-Size Arrays

Arrays of any supported primitive type:

| Pattern | Registers | Calculation |
|---------|-----------|-------------|
| `[u16; N]` | N | N × 1 |
| `[u32; N]` | N × 2 | N × 2 |
| `[f32; N]` | N × 2 | N × 2 |
| `[f64; N]` | N × 4 | N × 4 |

**Example**:
```rust
#[derive(ModbusMapper)]
struct ArrayData {
    #[modbus(address = 0)]
    samples: [u16; 10],      // Registers 0-9

    #[modbus(address = 10)]
    measurements: [f32; 5],  // Registers 10-19 (5 × 2)
}
```

**Register count**: 20 total

**Rules**:
- ✅ Array length must be compile-time constant
- ✅ All elements mapped sequentially
- ❌ No nested arrays: `[[u16; 2]; 3]` is NOT supported (v1.0)

---

### 6. Fixed-Length Strings

Strings with compile-time specified maximum length:

| Encoding | Bytes per Register | Max Chars per Register |
|----------|-------------------|------------------------|
| ASCII | 2 | 2 |
| UTF-8 | 2 | 1-2 (variable) |

**Storage format**: 2 ASCII/UTF-8 bytes per 16-bit register (big-endian byte order)

**Example**:
```rust
#[derive(ModbusMapper)]
struct DeviceInfo {
    // 16 registers = 32 bytes max
    #[modbus(address = 0, length = 16, encoding = "ascii", padding = "null")]
    device_name: String,

    // 8 registers = 16 bytes max
    #[modbus(address = 16, length = 8, encoding = "ascii", padding = "space")]
    serial_number: String,

    #[modbus(address = 24)]
    firmware_version: u16,
}
```

**Attribute options**:
- `length = N` - **Required**: Number of registers to allocate (N registers = 2N bytes)
- `encoding = "ascii"` - Optional: Character encoding (default: "ascii")
  - `"ascii"` - ASCII only (0-127), invalid chars become '?'
  - `"utf8"` - Full UTF-8 support
- `padding = "null"` - Optional: How to pad short strings (default: "null")
  - `"null"` - Null bytes (0x00) for remainder
  - `"space"` - Space chars (0x20) for remainder
  - `"none"` - Leave remainder unchanged (read existing values)

**Behavior**:
- **Write**: String is truncated if too long, padded if too short
- **Read**: String is read until null terminator or max length
- **Validation**: Compile error if `length` attribute missing

**Register layout example**:
```rust
// device_name = "PLC-1" with length=4, padding="null"
// Registers: ['PL', 'C-', '1\0', '\0\0']
// Register 0: 0x504C (P=0x50, L=0x4C)
// Register 1: 0x432D (C=0x43, -=0x2D)
// Register 2: 0x3100 (1=0x31, \0=0x00)
// Register 3: 0x0000 (\0=0x00, \0=0x00)
```

**Rules**:
- ✅ Length must be specified at compile time
- ✅ Strings exceeding length are truncated (no error)
- ✅ Empty strings write padding characters
- ⚠️ UTF-8 multi-byte chars may be truncated mid-sequence if length too small
- ❌ No dynamic-length strings (must specify `length`)

---

### 7. Option<T> (Nullable Types)

Optional values with configurable None representation:

| Strategy | Attribute | Description |
|----------|-----------|-------------|
| Sentinel Value | `none_value = N` | Specific value means None |
| NaN | `none_value = "nan"` | For f32/f64, use NaN |
| Separate Flag | `none_flag = addr` | Separate bool register |

**Example - Sentinel values**:
```rust
#[derive(ModbusMapper)]
struct Sensor {
    // 0xFFFF means None
    #[modbus(address = 0, none_value = 0xFFFF)]
    optional_reading: Option<u16>,

    // NaN means None (for floats)
    #[modbus(address = 1, none_value = "nan")]
    optional_temp: Option<f32>,

    // 0 means None, -1 to match device behavior
    #[modbus(address = 3, none_value = 0)]
    optional_count: Option<i16>,
}
```

**Example - Separate validity flags**:
```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Data {
    #[modbus(address = 0, none_flag = 100)]  // Validity bit at register 100, bit 0
    value1: Option<u16>,

    #[modbus(address = 1, none_flag = 100, none_flag_bit = 1)]  // Bit 1 of register 100
    value2: Option<u16>,
}
```

**Rules**:
- ✅ Must specify either `none_value` or `none_flag` attribute
- ✅ `none_value = "nan"` only valid for f32/f64
- ✅ Sentinel value must be valid for inner type
- ✅ `none_flag` can reference same register for multiple fields (different bits)

---

### 8. Enums with #[repr]

Enums with explicit representation:

```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Controller {
    #[modbus(address = 0)]
    mode: OperationMode,

    #[modbus(address = 1)]
    status: DeviceStatus,
}

// Maps to u16 register
#[derive(ModbusEnum)]
#[repr(u16)]
enum OperationMode {
    Idle = 0,
    Running = 1,
    Maintenance = 2,
    Error = 3,
}

// Maps to u8 (uses lower byte of register)
#[derive(ModbusEnum)]
#[repr(u8)]
enum DeviceStatus {
    Stopped = 0,
    Starting = 1,
    Active = 2,
}
```

**Supported representations**:
- `#[repr(u8)]` - 1 register (lower byte)
- `#[repr(u16)]` - 1 register
- `#[repr(u32)]` - 2 registers
- `#[repr(u64)]` - 4 registers
- `#[repr(i8)]`, `#[repr(i16)]`, `#[repr(i32)]`, `#[repr(i64)]` - Signed variants

**Validation**:
- Read validates discriminant is valid variant
- Invalid discriminant returns `ModbusMapperError::InvalidEnum`
- Compile error if enum has no `#[repr]` attribute

---

### 9. Bit Fields

Pack multiple boolean flags into a single register:

```rust
#[derive(ModbusMapper)]
struct StatusFlags {
    #[modbus(address = 0, bit = 0)]
    motor_running: bool,

    #[modbus(address = 0, bit = 1)]
    pump_active: bool,

    #[modbus(address = 0, bit = 2)]
    alarm_active: bool,

    #[modbus(address = 0, bit = 3)]
    maintenance_mode: bool,

    #[modbus(address = 0, bits = 4..8)]  // Bits 4-7 (4 bits)
    error_code: u8,

    #[modbus(address = 1)]
    temperature: i16,
}
```

**Rules**:
- ✅ Multiple fields can share same register address
- ✅ `bit = N` for single bit (0-15)
- ✅ `bits = N..M` for bit range (returns u8/u16 depending on size)
- ✅ Can mix bit fields with regular fields at different addresses
- ❌ Cannot have overlapping bit ranges
- ❌ Cannot use coil register type with bit fields (use holding/input)

**Register layout**:
```
Register 0: [bit15][bit14]...[alarm:bit2][pump:bit1][motor:bit0]
Register 1: temperature (full 16 bits)
```

---

### 10. Nested Structs (Flattening)

Compose structs from other ModbusMapper structs:

```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0)]
struct Point3D {
    #[modbus(address = 0)]
    x: f32,  // Registers 0-1

    #[modbus(address = 2)]
    y: f32,  // Registers 2-3

    #[modbus(address = 4)]
    z: f32,  // Registers 4-7
}

#[derive(ModbusMapper)]
struct RobotState {
    #[modbus(address = 0, flatten)]
    position: Point3D,      // Registers 0-5

    #[modbus(address = 6, flatten)]
    velocity: Point3D,      // Registers 6-11

    #[modbus(address = 12)]
    timestamp: u32,         // Registers 12-13
}
```

**Rules**:
- ✅ Nested struct must also derive `ModbusMapper`
- ✅ `flatten` attribute causes fields to be inline
- ✅ Addresses in nested struct are relative to specified address
- ✅ Can nest multiple levels deep
- ✅ Register type must match between parent and nested struct

---

### 11. Tuples

Fixed-size tuple types:

```rust
#[derive(ModbusMapper)]
struct Measurements {
    // Tuple: first element at address, second at address+1
    #[modbus(address = 0)]
    min_max: (u16, u16),    // Registers 0-1

    #[modbus(address = 2)]
    coordinates: (f32, f32), // Registers 2-5 (2 registers each)
}
```

**Supported tuple elements**:
- Any supported primitive type
- Tuples up to 8 elements: `(T1, T2, ..., T8)`
- Elements mapped sequentially

**Rules**:
- ✅ All elements must be supported types
- ✅ Total register count = sum of element register counts
- ❌ No nested tuples: `((u16, u16), u16)` not supported

---

### 12. Multi-dimensional Arrays

Fixed-size multi-dimensional arrays:

```rust
#[derive(ModbusMapper)]
struct ImageSensor {
    // 2D array: 10 rows × 8 columns = 80 registers
    #[modbus(address = 0)]
    pixel_data: [[u16; 8]; 10],

    // 3D array
    #[modbus(address = 100)]
    rgb_cube: [[[u8; 4]; 4]; 3],  // 3×4×4 = 48 elements, 48 registers
}
```

**Rules**:
- ✅ Up to 3 dimensions: `[[[T; Z]; Y]; X]`
- ✅ Layout is row-major (last index varies fastest)
- ✅ Total registers = product of all dimensions × registers per element

---

## ❌ TRULY UNSUPPORTED TYPES (Version 1.0)

### 1. Dynamic-Size Types

**NOT SUPPORTED - No compile-time size**:

```rust
Vec<T>           // Dynamic length vector
Box<T>           // Heap pointer
Rc<T>, Arc<T>    // Reference counted pointers
Cow<T>           // Clone-on-write
```

**Why**: Modbus requires knowing exact register count at compile time. These types can grow or shrink.

**Note**: `String` IS supported but requires explicit `length` attribute (see above)

---

### 2. References and Pointers

**NOT SUPPORTED**:

```rust
&T               // Borrow
&mut T           // Mutable borrow
*const T         // Raw pointer
*mut T           // Mutable raw pointer
```

**Why**: Modbus maps values, not references. References don't own data.

---

### 3. Zero-Sized Types

**NOT SUPPORTED**:

```rust
()               // Unit type
PhantomData<T>   // Zero-size marker
```

**Why**: Don't map to any registers. Use `#[modbus(skip)]` for non-mapped fields.

---

## 🔧 SPECIAL ATTRIBUTES

### Skip Fields

Fields that should not be mapped to Modbus:

```rust
#[derive(ModbusMapper)]
struct Controller {
    #[modbus(address = 0)]
    setpoint: f32,

    #[modbus(skip)]
    cached_calculation: f64,  // Not sent to Modbus

    #[modbus(skip)]
    internal_state: String,   // Can be any type
}
```

**Use case**: Internal state, cached values, metadata

---

## 📊 REGISTER TYPE CONSTRAINTS

Structs must use **one** register type:

| Register Type | Allowed Field Types | Access |
|---------------|---------------------|--------|
| `holding` | All numeric types (u8-u64, i8-i64, f32, f64) | Read/Write |
| `input` | All numeric types | Read-only |
| `coil` | `bool` only | Read/Write |
| `discrete` | `bool` only | Read-only |

**Invalid mixing**:
```rust
// ❌ ERROR: Cannot mix bool with numeric types
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Mixed {
    temperature: u16,  // Holding register
    alarm: bool,       // Needs coil!
}
```

**Solution**: Use separate structs
```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct NumericData {
    #[modbus(address = 0)]
    temperature: u16,
}

#[derive(ModbusMapper)]
#[modbus(register_type = "coil")]
struct BooleanFlags {
    #[modbus(address = 0)]
    alarm: bool,
}
```

---

## 🎯 VALIDATION RULES

The macro will enforce these at **compile time**:

1. ✅ All field types are supported (primitives, strings, arrays, Option, enums, tuples, nested)
2. ✅ All addresses are specified (no auto-allocation in v1.0)
3. ✅ No address overlaps between fields (except bit fields sharing same register)
4. ✅ All fields use compatible register types
5. ✅ Array/tuple lengths are compile-time constants
6. ✅ String fields must have `length` attribute specified
7. ✅ Option<T> fields must have `none_value` or `none_flag` attribute
8. ✅ Enum fields must have `#[repr(u8/u16/u32/u64)]` attribute
9. ✅ Nested struct fields must have `flatten` attribute
10. ✅ Bit field ranges don't overlap within same register
11. ✅ Endianness only on multi-register types (u32, u64, f32, f64, Option<multi-reg>)
12. ✅ `bool` fields only in coil/discrete structs OR as bit fields in holding/input

**Example validation errors**:
```rust
#[derive(ModbusMapper)]
struct Invalid {
    #[modbus(address = 0)]
    field1: u32,             // Registers 0-1

    #[modbus(address = 1)]   // ❌ ERROR: Overlaps with field1!
    field2: u16,

    #[modbus(address = 3, endian = "big")]
    field3: u16,             // ❌ ERROR: u16 doesn't need endian config!
}
```

---

## 📈 FUTURE EXTENSIONS (v2.0+)

Additional features that could be added in future versions:

### 1. Custom Types via Trait
```rust
trait ModbusSerialize {
    fn to_registers(&self) -> Vec<u16>;
    fn from_registers(regs: &[u16]) -> Result<Self, Error>;
    fn register_count() -> u16;
}

// User implements for custom type (e.g., complex number, UUID, etc.)
impl ModbusSerialize for MyCustomType { ... }
```

### 2. Automatic Address Allocation
```rust
#[derive(ModbusMapper)]
#[modbus(auto_allocate)]  // Automatically assign sequential addresses
struct AutoLayout {
    temperature: f32,    // Auto-assigned to 0-1
    pressure: f32,       // Auto-assigned to 2-3
    status: u16,         // Auto-assigned to 4
}
```

### 3. Dynamic String Length Prefix
```rust
// First register = length, following registers = data
#[modbus(address = 0, length = "prefixed", max_length = 32)]
variable_string: String,
```

### 4. Computed/Derived Fields
```rust
#[modbus(address = 0)]
celsius: f32,

#[modbus(computed = "celsius * 9.0 / 5.0 + 32.0")]
fahrenheit: f32,  // Read-only, computed on access
```

### 5. Conditional Fields
```rust
#[modbus(address = 0)]
mode: OperationMode,

#[modbus(address = 1, when = "mode == OperationMode::Advanced")]
advanced_param: f32,  // Only serialize when mode is Advanced
```

### 6. Versioning Support
```rust
#[derive(ModbusMapper)]
#[modbus(version = 2)]
struct ConfigV2 {
    #[modbus(since = 1)]
    old_field: u16,

    #[modbus(since = 2)]
    new_field: f32,
}
```

---

## 🔄 CLIENT AND SERVER SUPPORT

The crate supports both Modbus **client** (master) and **server** (slave) use cases:

### Client Mode (Reading from Modbus Devices)

```rust
#[derive(ModbusMapper)]
#[modbus(base_address = 0, register_type = "holding")]
struct SensorData {
    #[modbus(address = 0)]
    temperature: f32,

    #[modbus(address = 2)]
    pressure: u16,
}

// Read entire struct from Modbus device
let data = SensorData::read_from_modbus(&mut ctx).await?;

// Read specific field
let mut data = SensorData::default();
data.read_field_from_modbus(&mut ctx, "temperature").await?;
```

### Server Mode (Responding to Modbus Requests)

```rust
#[derive(ModbusMapper, Default)]
#[modbus(base_address = 0, register_type = "holding")]
struct DeviceState {
    #[modbus(address = 0)]
    setpoint: f32,

    #[modbus(address = 2, readonly)]
    current_value: f32,

    #[modbus(address = 4)]
    mode: OperationMode,
}

// Serve requests
let mut state = DeviceState::default();

// Handle read request from client
let response_registers = state.to_registers();

// Handle write request from client
state.update_from_registers(&incoming_registers)?;

// Or update specific field from write request
state.update_field_from_registers("setpoint", &incoming_registers)?;
```

### Server-Specific Attributes

```rust
#[derive(ModbusMapper)]
struct ServerConfig {
    // Read-only field - reject writes
    #[modbus(address = 0, readonly)]
    firmware_version: u16,

    // Write-only field - reads return 0
    #[modbus(address = 1, writeonly)]
    command: u16,

    // Validation on write
    #[modbus(address = 2, validate = "0..=100")]
    percentage: u8,

    // Custom validator function
    #[modbus(address = 3, validate_with = "validate_temperature")]
    temperature: i16,
}

fn validate_temperature(value: &i16) -> Result<(), String> {
    if *value < -40 || *value > 125 {
        Err(format!("Temperature {} out of range [-40, 125]", value))
    } else {
        Ok(())
    }
}
```

### Change Notifications (Server)

```rust
use modbus_mapper::ChangeCallback;

#[derive(ModbusMapper)]
#[modbus(on_change = "handle_change")]
struct MonitoredData {
    #[modbus(address = 0, on_change = "handle_setpoint_change")]
    setpoint: f32,

    #[modbus(address = 2)]
    value: f32,
}

fn handle_setpoint_change(old: &f32, new: &f32) {
    println!("Setpoint changed: {} → {}", old, new);
}

fn handle_change(field: &str, data: &MonitoredData) {
    println!("Field '{}' changed", field);
}
```

### Thread Safety (Server)

For multi-threaded servers, use `Arc<Mutex<T>>` or `Arc<RwLock<T>>`:

```rust
use std::sync::{Arc, RwLock};

#[derive(ModbusMapper, Default)]
struct SharedState {
    #[modbus(address = 0)]
    value: u16,
}

// In server:
let state = Arc::new(RwLock::new(SharedState::default()));

// Read handler
let state_clone = state.clone();
let read_handler = move |addr, count| {
    let guard = state_clone.read().unwrap();
    guard.read_registers(addr, count)
};

// Write handler
let state_clone = state.clone();
let write_handler = move |addr, values| {
    let mut guard = state_clone.write().unwrap();
    guard.write_registers(addr, values)
};
```

---

## 📝 SUMMARY

### ✅ Supported (v1.0)
- **Primitive integers**: `u8, u16, u32, u64, i8, i16, i32, i64`
- **Floating point**: `f32, f64` (with configurable endianness)
- **Boolean**: `bool` (in dedicated coil/discrete structs, or as bit fields)
- **Fixed-length strings**: `String` (with required `length` attribute)
- **Fixed-size arrays**: `[T; N]` where T is any supported type
- **Multi-dimensional arrays**: Up to 3D arrays `[[[T; Z]; Y]; X]`
- **Option<T>**: Nullable types with sentinel values or separate validity flags
- **Enums**: `#[repr(u8/u16/u32/u64)]` enums with explicit discriminants
- **Bit fields**: Pack multiple `bool` or small integers into single registers
- **Nested structs**: Flattening with `#[modbus(flatten)]` attribute
- **Tuples**: Up to 8-element tuples of supported types

### ❌ Truly Unsupported (v1.0)
- **Dynamic types**: `Vec<T>, Box<T>, Rc<T>, Arc<T>, Cow<T>` (runtime-sized)
- **References**: `&T, &mut T, *const T, *mut T` (don't own data)
- **Zero-sized types**: `(), PhantomData<T>` (don't map to registers)

### 🎯 Design Philosophy
**"If it has a compile-time known size and deterministic layout, it's supported."**

This comprehensive type support covers **99%+ of industrial Modbus use cases** including:
- ✅ Simple sensor readings (primitives)
- ✅ Complex multi-field devices (structs with many types)
- ✅ Optional values (Option<T> with configurable None representation)
- ✅ State machines (enums)
- ✅ Status flags (bit fields)
- ✅ Device metadata (strings)
- ✅ Array data (sensor arrays, image data, etc.)
- ✅ Structured data (nested structs, tuples)
