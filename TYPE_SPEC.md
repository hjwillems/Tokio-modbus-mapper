# Type Specification — What Maps to Modbus

This document is the source of truth for what the crate can map **today** versus what
is **planned**. If a feature isn't in the "Supported now" section, it is not implemented
yet — using it is either a compile error or simply unavailable.

## Design constraints

Modbus operates on fixed-size, contiguous register blocks. Every supported type must have:

1. **Compile-time known size** — the exact register count is known when the macro runs.
2. **Deterministic layout** — the same value always maps to the same registers.
3. **No runtime indirection** — values map directly to registers, no heap pointers.

The crate is a thin layer over `tokio-modbus`: the derive macro generates the
register conversion at compile time, and a small async layer (`ModbusRead`/`ModbusWrite`)
performs the I/O.

---

## ✅ Supported now (v0.1)

### Numeric primitives

| Type | Registers | Endianness | Layout |
|------|-----------|------------|--------|
| `u8` / `i8` | 1 | — | Value in low byte; high byte zero (unless byte-packed) |
| `u16` / `i16` | 1 | — | Direct single-register mapping |
| `u32` / `i32` | 2 | configurable | Two registers, word order per `endian` |
| `u64` / `i64` | 4 | configurable | Four registers, word order per `endian` |
| `f32` | 2 | configurable | IEEE 754 single, word order per `endian` |
| `f64` | 4 | configurable | IEEE 754 double, word order per `endian` |

Each 16-bit register is big-endian on the wire (Modbus network byte order). For
multi-register types, the **word order** is configurable:

- `big` (default): most-significant word first — `f32` → `[MSW, LSW]`
- `little`: least-significant word first — `f32` → `[LSW, MSW]`

```rust
#[derive(ModbusMapper)]
#[modbus(default_endian = "big")]
struct FloatData {
    #[modbus(address = 0, endian = "big")]
    flow_rate: f32,        // registers 0-1, big-endian word order

    #[modbus(address = 2, endian = "little")]
    temperature: f32,      // registers 2-3, little-endian word order

    #[modbus(address = 4)]
    precise_value: f64,    // registers 4-7
}
```

### Booleans

A `bool` occupies a full register (`0` or `1`) unless it is bit-packed (below).

```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Status {
    #[modbus(address = 0)]
    enabled: bool,         // register 0: 0x0000 or 0x0001
}
```

### Bit packing

Pack up to 16 `bool` fields into the individual bits of one register by giving them
the same `address` and distinct `bit` positions (0-15):

```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Flags {
    #[modbus(address = 0, bit = 0)] motor_running: bool,
    #[modbus(address = 0, bit = 1)] pump_active: bool,
    #[modbus(address = 0, bit = 2)] alarm: bool,
    #[modbus(address = 0, bit = 15)] fault: bool,
    // all four packed into register 0
}
```

Rules:
- `bit` must be 0-15 and unique within a register (checked at compile time).
- `bit` is only valid on `bool` fields.

### Byte packing

Pack two `u8`/`i8` values into the high and low byte of one register:

```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Packed {
    #[modbus(address = 0, offset = "high")] error_code: u8,   // bits 8-15
    #[modbus(address = 0, offset = "low")]  status_code: u8,  // bits 0-7
}
```

Rules:
- `offset` is `"high"` or `"low"`, only valid on `u8`/`i8`.
- At most one `high` and one `low` per register (checked at compile time).
- A field cannot have both `bit` and `offset`.

### Register types

| `register_type` | Field types | Access | Read FC | Write FC |
|-----------------|-------------|--------|---------|----------|
| `holding` | all numeric + packing | read/write | `0x03` | `0x10` |
| `input` | all numeric + packing | read-only | `0x04` | — |

`coil` and `discrete` are **not supported yet** (see below).

### Addressing rule (important)

Field `address` is an **offset within the struct's register block**, not an arbitrary
label. The generated buffer is contiguous, so:

- Addresses must start at `0` and be **gap-free** (packed fields share one address).
- Gaps or overlaps are a **compile error**.
- `base_address` is the absolute address of the block on the wire, used by the I/O layer.

To model a device whose map has reserved gaps, split it into multiple structs, each
mapping one contiguous region, with the appropriate `base_address`.

```rust
// ❌ Compile error: b at address 5 leaves a gap after a (which ends at 1)
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Bad {
    #[modbus(address = 0)] a: u16,
    #[modbus(address = 5)] b: u16,
}
```

### Skipped fields

`#[modbus(skip)]` excludes a field from the mapping entirely (any type allowed):

```rust
#[derive(ModbusMapper)]
#[modbus(register_type = "holding")]
struct Controller {
    #[modbus(address = 0)] setpoint: f32,
    #[modbus(skip)] cached: f64,      // not mapped
}
```

---

## 🚧 Not implemented yet (planned)

These are **design sketches**, not working features. The syntax shown is provisional and
may change. Using them today will not compile or is unavailable.

- **`String`** — fixed-length, with a required `length` attribute and configurable
  encoding/padding.
- **`Option<T>`** — nullable values via a sentinel `none_value` or a separate validity bit.
- **Enums (`#[derive(ModbusEnum)]`)** — `#[repr(...)]` enums with discriminant validation.
  *(The `ModbusEnum` derive currently exists only as a stub.)*
- **Fixed-size arrays `[T; N]`** and **multi-dimensional arrays**.
- **Tuples** — `(T1, T2, ...)` mapped sequentially.
- **Nested structs** — composing mappings via a `flatten` attribute.
- **`coil` / `discrete` register types** — these are bit-addressed on the wire
  (`Vec<bool>`), which the current `Vec<u16>` model cannot represent. Rejected at compile
  time for now; use bit-packing into a `holding` block to model boolean flags.
- **Server mode** — `readonly`/`writeonly` fields, per-field write validation, change
  callbacks, and request handlers.

---

## ❌ Truly unsupported (by design)

These cannot satisfy the Modbus constraints and are not planned:

- **Dynamically-sized types**: `Vec<T>`, `Box<T>`, `Rc<T>`, `Arc<T>`, `Cow<T>` — no
  compile-time size. (`String` will be supported only with an explicit fixed `length`.)
- **References / pointers**: `&T`, `&mut T`, `*const T`, `*mut T` — they don't own data.
- **Zero-sized types**: `()`, `PhantomData<T>` — map to no registers. Use `#[modbus(skip)]`
  for fields you don't want mapped.

---

## Validation summary

Enforced at **compile time** today:

1. Field addresses form a gap-free, contiguous block starting at 0.
2. `bit` is 0-15, unique per register, and only on `bool`.
3. `offset` is `"high"`/`"low"`, only on `u8`/`i8`, at most one of each per register.
4. A field cannot combine `bit` and `offset`.
5. Normal fields cannot share a register with packed fields.
6. `register_type` must be `holding` or `input`.
7. Non-skipped fields must declare an `address`.

Enforced at **runtime**:

- `from_registers` rejects a slice whose length doesn't match `register_count()`
  (`RegisterCountMismatch`).
- I/O on an unsupported register type / direction returns `UnsupportedRegisterType`.
- Modbus transport errors and exception responses surface as `Transport` / `Exception`.
