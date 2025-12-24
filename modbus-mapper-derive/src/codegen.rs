//! Code generation for ModbusMapper derive macro.

use crate::parse::{ModbusFieldOpts, ModbusMapperOpts};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Type, TypePath};

/// Generate the complete implementation for a ModbusMapper struct.
pub fn generate_modbus_mapper_impl(opts: &ModbusMapperOpts) -> TokenStream {
    let fields = opts.fields();

    let to_registers_impl = generate_to_registers(&fields, opts);
    let from_registers_impl = generate_from_registers(&fields, opts);
    let metadata_impl = generate_metadata(&fields, opts);

    quote! {
        #to_registers_impl
        #from_registers_impl
        #metadata_impl
    }
}

/// Generate the `ToRegisters` trait implementation.
fn generate_to_registers(fields: &[&ModbusFieldOpts], opts: &ModbusMapperOpts) -> TokenStream {
    let struct_name = &opts.ident;
    let default_endian = opts.get_default_endian();

    let mut field_conversions = vec![];

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let endian = field.get_endian(&default_endian);

        let conversion = generate_field_to_registers(field_name, field_type, &endian);
        field_conversions.push(conversion);
    }

    let total_register_count = calculate_total_register_count(fields);

    quote! {
        impl ::modbus_mapper::ToRegisters for #struct_name {
            fn to_registers(&self) -> Vec<u16> {
                let mut registers = Vec::with_capacity(Self::register_count() as usize);

                #(#field_conversions)*

                registers
            }

            fn register_count() -> u16 {
                #total_register_count
            }
        }
    }
}

/// Generate code to convert a single field to registers.
fn generate_field_to_registers(
    field_name: &syn::Ident,
    field_type: &Type,
    endian: &str,
) -> TokenStream {
    let type_str = type_to_string(field_type);

    match type_str.as_str() {
        "u8" | "i8" => {
            quote! {
                registers.push(self.#field_name as u16);
            }
        }
        "u16" | "i16" => {
            quote! {
                registers.push(self.#field_name as u16);
            }
        }
        "u32" | "i32" => {
            let endian_enum = endian_to_enum(endian);
            let convert_fn = if type_str == "u32" {
                quote! { ::modbus_mapper::endian::u32_to_registers }
            } else {
                quote! { ::modbus_mapper::endian::i32_to_registers }
            };
            quote! {
                {
                    let regs = #convert_fn(self.#field_name, #endian_enum);
                    registers.extend_from_slice(&regs);
                }
            }
        }
        "u64" | "i64" => {
            let endian_enum = endian_to_enum(endian);
            let convert_fn = if type_str == "u64" {
                quote! { ::modbus_mapper::endian::u64_to_registers }
            } else {
                quote! { ::modbus_mapper::endian::i64_to_registers }
            };
            quote! {
                {
                    let regs = #convert_fn(self.#field_name, #endian_enum);
                    registers.extend_from_slice(&regs);
                }
            }
        }
        "f32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                {
                    let regs = ::modbus_mapper::endian::f32_to_registers(self.#field_name, #endian_enum);
                    registers.extend_from_slice(&regs);
                }
            }
        }
        "f64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                {
                    let regs = ::modbus_mapper::endian::f64_to_registers(self.#field_name, #endian_enum);
                    registers.extend_from_slice(&regs);
                }
            }
        }
        "bool" => {
            quote! {
                registers.push(if self.#field_name { 1 } else { 0 });
            }
        }
        _ => {
            // For now, generate a compile error for unsupported types
            quote! {
                compile_error!(concat!("Unsupported type for field '", stringify!(#field_name), "': ", stringify!(#field_type)));
            }
        }
    }
}

/// Generate the `FromRegisters` trait implementation.
fn generate_from_registers(fields: &[&ModbusFieldOpts], opts: &ModbusMapperOpts) -> TokenStream {
    let struct_name = &opts.ident;
    let default_endian = opts.get_default_endian();

    let mut field_deserializations = vec![];
    let mut offset = 0u16;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let endian = field.get_endian(&default_endian);
        let register_count = get_type_register_count(&type_to_string(field_type));

        let deserialization = generate_field_from_registers(
            field_name,
            field_type,
            &endian,
            offset,
        );

        field_deserializations.push(deserialization);
        offset += register_count;
    }

    quote! {
        impl ::modbus_mapper::FromRegisters for #struct_name {
            fn from_registers(registers: &[u16]) -> ::modbus_mapper::Result<Self> {
                if registers.len() != Self::register_count() as usize {
                    return Err(::modbus_mapper::ModbusMapperError::RegisterCountMismatch {
                        expected: Self::register_count() as usize,
                        actual: registers.len(),
                    });
                }

                Ok(Self {
                    #(#field_deserializations,)*
                })
            }
        }
    }
}

/// Generate code to deserialize a single field from registers.
fn generate_field_from_registers(
    field_name: &syn::Ident,
    field_type: &Type,
    endian: &str,
    offset: u16,
) -> TokenStream {
    let type_str = type_to_string(field_type);
    let offset_lit = syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());

    match type_str.as_str() {
        "u8" => {
            quote! {
                #field_name: registers[#offset_lit] as u8
            }
        }
        "i8" => {
            quote! {
                #field_name: registers[#offset_lit] as i8
            }
        }
        "u16" => {
            quote! {
                #field_name: registers[#offset_lit]
            }
        }
        "i16" => {
            quote! {
                #field_name: registers[#offset_lit] as i16
            }
        }
        "u32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::u32_from_registers(
                    &registers[#offset_lit..#offset_lit + 2],
                    #endian_enum
                )
            }
        }
        "i32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::i32_from_registers(
                    &registers[#offset_lit..#offset_lit + 2],
                    #endian_enum
                )
            }
        }
        "u64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::u64_from_registers(
                    &registers[#offset_lit..#offset_lit + 4],
                    #endian_enum
                )
            }
        }
        "i64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::i64_from_registers(
                    &registers[#offset_lit..#offset_lit + 4],
                    #endian_enum
                )
            }
        }
        "f32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::f32_from_registers(
                    &registers[#offset_lit..#offset_lit + 2],
                    #endian_enum
                )
            }
        }
        "f64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::f64_from_registers(
                    &registers[#offset_lit..#offset_lit + 4],
                    #endian_enum
                )
            }
        }
        "bool" => {
            quote! {
                #field_name: registers[#offset_lit] != 0
            }
        }
        _ => {
            quote! {
                #field_name: compile_error!(concat!("Unsupported type: ", stringify!(#field_type)))
            }
        }
    }
}

/// Generate the `ModbusMetadata` trait implementation.
fn generate_metadata(fields: &[&ModbusFieldOpts], opts: &ModbusMapperOpts) -> TokenStream {
    let struct_name = &opts.ident;
    let base_address = opts.get_base_address();
    let register_type = opts.get_register_type();

    let register_type_expr = match register_type.as_str() {
        "holding" => quote! { ::modbus_mapper::RegisterType::Holding },
        "input" => quote! { ::modbus_mapper::RegisterType::Input },
        "coil" => quote! { ::modbus_mapper::RegisterType::Coil },
        "discrete" => quote! { ::modbus_mapper::RegisterType::Discrete },
        _ => quote! { ::modbus_mapper::RegisterType::Holding },
    };

    let mut field_address_arms = vec![];
    let mut field_count_arms = vec![];

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let address = field.address.unwrap();
        let type_str = type_to_string(&field.ty);
        let register_count = get_type_register_count(&type_str);

        field_address_arms.push(quote! {
            #field_name_str => Some(#address)
        });

        field_count_arms.push(quote! {
            #field_name_str => Some(#register_count)
        });
    }

    quote! {
        impl ::modbus_mapper::ModbusMetadata for #struct_name {
            fn base_address() -> u16 {
                #base_address
            }

            fn register_type() -> ::modbus_mapper::RegisterType {
                #register_type_expr
            }

            fn field_address(field_name: &str) -> Option<u16> {
                match field_name {
                    #(#field_address_arms,)*
                    _ => None,
                }
            }

            fn field_register_count(field_name: &str) -> Option<u16> {
                match field_name {
                    #(#field_count_arms,)*
                    _ => None,
                }
            }
        }
    }
}

/// Calculate the total register count for all fields.
fn calculate_total_register_count(fields: &[&ModbusFieldOpts]) -> u16 {
    fields
        .iter()
        .map(|field| {
            let type_str = type_to_string(&field.ty);
            get_type_register_count(&type_str)
        })
        .sum()
}

/// Get the number of registers required for a type.
fn get_type_register_count(type_str: &str) -> u16 {
    match type_str {
        "u8" | "i8" | "u16" | "i16" | "bool" => 1,
        "u32" | "i32" | "f32" => 2,
        "u64" | "i64" | "f64" => 4,
        _ => 0, // Unsupported type
    }
}

/// Convert a Type to a string representation.
fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            path.segments.last().unwrap().ident.to_string()
        }
        _ => String::new(),
    }
}

/// Convert endian string to Endianness enum.
fn endian_to_enum(endian: &str) -> TokenStream {
    match endian {
        "big" => quote! { ::modbus_mapper::Endianness::Big },
        "little" => quote! { ::modbus_mapper::Endianness::Little },
        _ => quote! { ::modbus_mapper::Endianness::Big },
    }
}
