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

## ❌ UNSUPPORTED TYPES (Version 1.0)

### 1. Dynamic-Size Types

**NOT SUPPORTED - No compile-time size**:

```rust
String           // Heap-allocated, unknown size
Vec<T>           // Dynamic length
Box<T>           // Heap pointer
Rc<T>, Arc<T>    // Reference counted pointers
Cow<T>           // Clone-on-write
```

**Why**: Modbus requires knowing exact register count at compile time. These types can grow or shrink.

**Alternative** (Future):
```rust
// Possible in v2.0 with explicit length:
#[modbus(address = 0, length = 16)]  // Fixed 16 registers = 32 bytes
device_name: String,
```

---

### 2. Option<T>

**NOT SUPPORTED - Ambiguous representation**:

```rust
Option<u16>      // How to represent None?
Option<f32>      // Use 0? NaN? Invalid value?
```

**Why**: No standard way to represent "None" in Modbus. Different industries use different conventions.

**Workarounds**:
```rust
// Option 1: Use separate validity flag
#[derive(ModbusMapper)]
struct WithFlag {
    #[modbus(address = 0)]
    value: u16,

    #[modbus(address = 1)]
    value_valid: bool,   // ❌ Can't mix types! Need separate struct
}

// Option 2: Use special value (manual handling)
const INVALID: u16 = 0xFFFF;

#[derive(ModbusMapper)]
struct Sensor {
    #[modbus(address = 0)]
    reading: u16,  // 0xFFFF means "not available"
}
```

**Future**: Could add `#[modbus(nullable = 0xFFFF)]` attribute in v2.0

---

### 3. Enums

**NOT SUPPORTED - Requires encoding rules**:

```rust
enum State {
    Idle,
    Running,
    Error,
}
```

**Why**: Need to define discriminant encoding, size, validation rules.

**Workarounds**:
```rust
// Use explicit numeric type + constants
const STATE_IDLE: u16 = 0;
const STATE_RUNNING: u16 = 1;
const STATE_ERROR: u16 = 2;

#[derive(ModbusMapper)]
struct Machine {
    #[modbus(address = 0)]
    state: u16,  // Manually encode enum as integer
}

// Or use #[repr(u16)] enum + manual conversion (future)
```

**Future**: Could support `#[repr(u16)]` enums with `#[derive(ModbusEnum)]` in v2.0

---

### 4. Nested Structs

**NOT SUPPORTED** (Version 1.0):

```rust
struct Inner {
    x: u16,
    y: u16,
}

#[derive(ModbusMapper)]
struct Outer {
    #[modbus(address = 0)]
    position: Inner,  // ❌ NOT SUPPORTED
}
```

**Why**: Adds complexity to address calculation and layout. Need to decide on flattening strategy.

**Workaround**: Flatten manually
```rust
#[derive(ModbusMapper)]
struct Outer {
    #[modbus(address = 0)]
    position_x: u16,

    #[modbus(address = 1)]
    position_y: u16,
}
```

**Future**: Could support with `#[modbus(flatten)]` attribute in v2.0

---

### 5. References and Pointers

**NOT SUPPORTED**:

```rust
&T               // Borrow
&mut T           // Mutable borrow
*const T         // Raw pointer
*mut T           // Mutable raw pointer
```

**Why**: Modbus maps values, not references. References don't own data.

---

### 6. Zero-Sized Types

**NOT SUPPORTED**:

```rust
()               // Unit type
PhantomData<T>   // Zero-size marker
```

**Why**: Don't map to any registers. Use `#[modbus(skip)]` for non-mapped fields.

---

### 7. Tuples

**NOT SUPPORTED**:

```rust
(u16, u16)       // Tuple
(f32, bool)      // Mixed tuple
```

**Why**: Use explicit struct fields instead for clarity.

**Workaround**: Define a proper struct
```rust
struct Point {
    x: u16,
    y: u16,
}
```

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

1. ✅ All field types are supported primitives or arrays thereof
2. ✅ All addresses are specified (no auto-allocation in v1.0)
3. ✅ No address overlaps between fields
4. ✅ All fields use compatible register types
5. ✅ Array lengths are compile-time constants
6. ✅ Endianness only on multi-register types (u32, u64, f32, f64)
7. ✅ `bool` fields only in coil/discrete structs

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

Types that **could** be supported with more complex implementation:

### 1. Fixed-Length Strings
```rust
#[modbus(address = 0, length = 16)]  // 16 registers = 32 bytes
device_name: String,
```

### 2. Nullable Types
```rust
#[modbus(address = 0, nullable = 0xFFFF)]
optional_value: Option<u16>,
```

### 3. Enums with #[repr]
```rust
#[derive(ModbusEnum)]
#[repr(u16)]
enum Mode {
    Auto = 0,
    Manual = 1,
    Off = 2,
}
```

### 4. Nested Structs with Flattening
```rust
#[derive(ModbusMapper)]
struct Outer {
    #[modbus(address = 0, flatten)]
    inner: Inner,
}
```

### 5. Bit Fields
```rust
#[modbus(address = 0, bit = 0)]
flag1: bool,

#[modbus(address = 0, bit = 1)]
flag2: bool,
```

### 6. Custom Types via Trait
```rust
trait ModbusSerialize {
    fn to_registers(&self) -> Vec<u16>;
    fn from_registers(regs: &[u16]) -> Result<Self, Error>;
    fn register_count() -> u16;
}

// User implements for custom type
impl ModbusSerialize for MyCustomType { ... }
```

---

## 📝 SUMMARY

### ✅ Supported (v1.0)
- All primitive integers: `u8, u16, u32, u64, i8, i16, i32, i64`
- Floating point: `f32, f64`
- Boolean: `bool` (in dedicated coil/discrete structs)
- Fixed-size arrays: `[T; N]` where T is any supported primitive

### ❌ Unsupported (v1.0)
- Dynamic types: `String, Vec<T>, Box<T>`
- Nullable types: `Option<T>`
- Enums: `enum { ... }`
- Nested structs
- References: `&T, &mut T`
- Zero-sized types: `(), PhantomData<T>`
- Tuples: `(T, U)`

### 🎯 Design Philosophy
**"If it doesn't have a compile-time known size and deterministic layout, it's not supported."**

This keeps the implementation simple, predictable, and type-safe while covering 95% of industrial Modbus use cases.
