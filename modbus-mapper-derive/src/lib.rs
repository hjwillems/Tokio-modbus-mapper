//! Procedural macros for modbus-mapper.
//!
//! This crate provides the `#[derive(ModbusMapper)]` and `#[derive(ModbusEnum)]` macros.

mod codegen;
mod parse;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use crate::codegen::generate_modbus_mapper_impl;
use crate::parse::{validate_field_attrs, validate_struct_attrs, ModbusMapperOpts};

/// Derive macro for mapping Rust structs to Modbus registers.
///
/// Generates [`ToRegisters`], [`FromRegisters`], and [`ModbusMetadata`] implementations
/// at compile time.
///
/// # Struct-level attributes
///
/// - `base_address = N` — absolute Modbus address of the block on the wire, used by the
///   async I/O layer (default: 0)
/// - `register_type = "TYPE"` — `"holding"` or `"input"` (default: `"holding"`).
///   `"coil"`/`"discrete"` are not supported yet.
/// - `default_endian = "ENDIAN"` — default word order: `"big"` or `"little"` (default: `"big"`)
///
/// # Field-level attributes
///
/// - `address = N` — offset of this field within the block (required unless `skip`).
///   Addresses must form a gap-free, contiguous layout starting at 0; this is checked
///   at compile time.
/// - `endian = "ENDIAN"` — override word order for this field: `"big"` or `"little"`
/// - `bit = B` — pack a `bool` into bit `B` (0-15) of the register at `address`
/// - `offset = "high" | "low"` — pack a `u8`/`i8` into the high/low byte of `address`
/// - `skip` — exclude this field from the Modbus mapping
///
/// # Example
///
/// ```ignore
/// use modbus_mapper::ModbusMapper;
///
/// #[derive(ModbusMapper)]
/// #[modbus(base_address = 0, register_type = "holding")]
/// struct SensorData {
///     #[modbus(address = 0)]
///     temperature: f32,
///
///     #[modbus(address = 2)]
///     pressure: u16,
/// }
/// ```
#[proc_macro_derive(ModbusMapper, attributes(modbus))]
pub fn derive_modbus_mapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse the attributes using darling
    let opts = match ModbusMapperOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    // Validate struct-level attributes
    if let Err(e) = validate_struct_attrs(&opts) {
        return TokenStream::from(e.write_errors());
    }

    // Validate field-level attributes
    let default_endian = opts.get_default_endian();
    for field in opts.fields() {
        if let Err(e) = validate_field_attrs(field, &default_endian) {
            return TokenStream::from(e.write_errors());
        }
    }

    // Generate the implementation
    let generated = generate_modbus_mapper_impl(&opts);

    TokenStream::from(generated)
}

/// Derive macro for mapping Rust enums to Modbus register values.
///
/// **Not implemented yet.** Deriving `ModbusEnum` is currently a compile error so it
/// fails loudly rather than appearing to work. Enum support (with `#[repr(...)]`
/// discriminant validation) is tracked in `TYPE_SPEC.md` under "Not implemented yet".
#[proc_macro_derive(ModbusEnum, attributes(modbus))]
pub fn derive_modbus_enum(input: TokenStream) -> TokenStream {
    // Reserve the name and the attribute, but reject use until it is real.
    let _ = parse_macro_input!(input as DeriveInput);

    let error = quote! {
        compile_error!(
            "#[derive(ModbusEnum)] is not implemented yet. Map the enum's underlying \
             integer type directly for now (e.g. a `u16` field) and convert manually. \
             See TYPE_SPEC.md for status."
        );
    };

    TokenStream::from(error)
}
