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
///
/// # Attributes
///
/// ## Struct-level attributes
///
/// - `base_address = N` - Base address for all fields (default: 0)
/// - `register_type = "TYPE"` - Register type: "holding", "input", "coil", or "discrete" (default: "holding")
/// - `default_endian = "ENDIAN"` - Default endianness: "big" or "little" (default: "big")
///
/// ## Field-level attributes
///
/// - `address = N` - Register address for this field (required unless `skip`)
/// - `endian = "ENDIAN"` - Endianness for this field: "big" or "little"
/// - `skip` - Skip this field in Modbus mapping
/// - `readonly` - Field is read-only (server mode)
/// - `writeonly` - Field is write-only (server mode)
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
/// The enum must have a `#[repr(u8/u16/u32/u64)]` attribute.
///
/// # Example
///
/// ```ignore
/// use modbus_mapper::ModbusEnum;
///
/// #[derive(ModbusEnum)]
/// #[repr(u16)]
/// enum OperationMode {
///     Idle = 0,
///     Running = 1,
///     Error = 2,
/// }
/// ```
#[proc_macro_derive(ModbusEnum, attributes(modbus))]
pub fn derive_modbus_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // TODO: Implement actual macro logic in Phase 6
    // For now, just generate a basic stub to test compilation

    let expanded = quote! {
        // Placeholder implementation
        impl #name {
            /// Placeholder method
            pub fn _placeholder() {
                unimplemented!("ModbusEnum derive macro not yet implemented")
            }
        }
    };

    TokenStream::from(expanded)
}
