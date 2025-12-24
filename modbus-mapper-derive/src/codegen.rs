//! Code generation for ModbusMapper derive macro.

use crate::parse::{ModbusFieldOpts, ModbusMapperOpts};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{Type, TypePath};

/// Represents how a field maps to register(s).
#[derive(Debug, Clone)]
enum FieldMapping {
    /// Normal field that occupies one or more complete registers.
    Normal {
        field: ModbusFieldOpts,
        endian: String,
        type_str: String,
        register_count: u16,
    },
    /// Boolean field packed into a specific bit position.
    BitPacked {
        field: ModbusFieldOpts,
        bit_position: u8,
    },
    /// u8/i8 field packed into high or low byte of a register.
    BytePacked {
        field: ModbusFieldOpts,
        is_high_byte: bool,
    },
}


/// A group of fields that share the same register address.
#[derive(Debug)]
struct RegisterGroup {
    address: u16,
    fields: Vec<FieldMapping>,
}

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

/// Group fields by their register addresses and create field mappings.
fn group_fields_by_address(
    fields: &[&ModbusFieldOpts],
    default_endian: &str,
) -> Vec<RegisterGroup> {
    let mut groups: BTreeMap<u16, Vec<FieldMapping>> = BTreeMap::new();

    for field in fields {
        let address = field.address.unwrap();
        let type_str = type_to_string(&field.ty);

        let mapping = if let Some(bit_pos) = field.bit {
            // Bit-packed boolean
            FieldMapping::BitPacked {
                field: (*field).clone(),
                bit_position: bit_pos,
            }
        } else if let Some(ref offset) = field.offset {
            // Byte-packed u8/i8
            let is_high = offset == "high";
            FieldMapping::BytePacked {
                field: (*field).clone(),
                is_high_byte: is_high,
            }
        } else {
            // Normal field
            let endian = field.get_endian(default_endian);
            let register_count = get_type_register_count(&type_str);
            FieldMapping::Normal {
                field: (*field).clone(),
                endian,
                type_str,
                register_count,
            }
        };

        groups.entry(address).or_insert_with(Vec::new).push(mapping);
    }

    // Convert BTreeMap to sorted Vec of RegisterGroups
    groups
        .into_iter()
        .map(|(address, fields)| RegisterGroup { address, fields })
        .collect()
}

/// Validate that fields in a register group are compatible.
fn validate_register_group(group: &RegisterGroup) -> Result<(), String> {
    if group.fields.is_empty() {
        return Ok(());
    }

    // Check if we have any normal fields
    let has_normal = group.fields.iter().any(|f| matches!(f, FieldMapping::Normal { .. }));
    let has_bit_packed = group.fields.iter().any(|f| matches!(f, FieldMapping::BitPacked { .. }));
    let has_byte_packed = group.fields.iter().any(|f| matches!(f, FieldMapping::BytePacked { .. }));

    // Normal fields cannot share a register with packed fields
    if has_normal && (has_bit_packed || has_byte_packed) {
        return Err(format!(
            "Register address {} mixes normal fields with packed fields. Normal fields must occupy their own register(s).",
            group.address
        ));
    }

    // Cannot mix bit-packed and byte-packed at same address
    if has_bit_packed && has_byte_packed {
        return Err(format!(
            "Register address {} mixes bit-packed and byte-packed fields. Choose one packing type per register.",
            group.address
        ));
    }

    // If we have multiple normal fields at the same address, they must all be multi-register types
    // starting at the same address (which is unusual but technically valid)
    if has_normal && group.fields.len() > 1 {
        return Err(format!(
            "Register address {} has multiple normal fields. Each normal field must have a unique starting address.",
            group.address
        ));
    }

    // Validate bit positions don't overlap
    if has_bit_packed {
        let mut bit_positions = std::collections::HashSet::new();
        for field in &group.fields {
            if let FieldMapping::BitPacked { bit_position, .. } = field {
                if !bit_positions.insert(bit_position) {
                    return Err(format!(
                        "Register address {} has duplicate bit position {}",
                        group.address, bit_position
                    ));
                }
            }
        }
    }

    // Validate byte packing - can have at most one high and one low
    if has_byte_packed {
        let mut has_high = false;
        let mut has_low = false;
        for field in &group.fields {
            if let FieldMapping::BytePacked { is_high_byte, .. } = field {
                if *is_high_byte {
                    if has_high {
                        return Err(format!(
                            "Register address {} has multiple fields in high byte",
                            group.address
                        ));
                    }
                    has_high = true;
                } else {
                    if has_low {
                        return Err(format!(
                            "Register address {} has multiple fields in low byte",
                            group.address
                        ));
                    }
                    has_low = true;
                }
            }
        }
    }

    Ok(())
}

/// Generate the `ToRegisters` trait implementation.
fn generate_to_registers(fields: &[&ModbusFieldOpts], opts: &ModbusMapperOpts) -> TokenStream {
    let struct_name = &opts.ident;
    let default_endian = opts.get_default_endian();

    let groups = group_fields_by_address(fields, &default_endian);

    // Validate all groups
    for group in &groups {
        if let Err(err) = validate_register_group(group) {
            return quote! {
                compile_error!(#err);
            };
        }
    }

    let register_conversions = groups.iter().map(|group| {
        generate_register_group_to_registers(group)
    });

    let total_register_count = calculate_total_register_count_from_groups(&groups);

    quote! {
        impl ::modbus_mapper::ToRegisters for #struct_name {
            fn to_registers(&self) -> Vec<u16> {
                let mut registers = Vec::with_capacity(Self::register_count() as usize);

                #(#register_conversions)*

                registers
            }

            fn register_count() -> u16 {
                #total_register_count
            }
        }
    }
}

/// Generate code to convert a register group to registers.
fn generate_register_group_to_registers(group: &RegisterGroup) -> TokenStream {
    if group.fields.is_empty() {
        return quote! {};
    }

    // Check what type of group this is
    let first = &group.fields[0];

    match first {
        FieldMapping::Normal { .. } => {
            // Normal field - should be only one in the group
            generate_normal_field_to_registers(&group.fields[0])
        }
        FieldMapping::BitPacked { .. } => {
            // Bit-packed fields - combine into one register
            generate_bit_packed_to_register(&group.fields)
        }
        FieldMapping::BytePacked { .. } => {
            // Byte-packed fields - combine into one register
            generate_byte_packed_to_register(&group.fields)
        }
    }
}

/// Generate code for a normal field.
fn generate_normal_field_to_registers(mapping: &FieldMapping) -> TokenStream {
    let FieldMapping::Normal { field, endian, type_str, .. } = mapping else {
        return quote! {};
    };

    let field_name = field.ident.as_ref().unwrap();

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
            let field_type = &field.ty;
            quote! {
                compile_error!(concat!("Unsupported type for field '", stringify!(#field_name), "': ", stringify!(#field_type)));
            }
        }
    }
}

/// Generate code to pack multiple bit fields into one register.
fn generate_bit_packed_to_register(fields: &[FieldMapping]) -> TokenStream {
    let bit_sets = fields.iter().map(|mapping| {
        if let FieldMapping::BitPacked { field, bit_position } = mapping {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                if self.#field_name {
                    register_value |= 1u16 << #bit_position;
                }
            }
        } else {
            quote! {}
        }
    });

    quote! {
        {
            let mut register_value = 0u16;
            #(#bit_sets)*
            registers.push(register_value);
        }
    }
}

/// Generate code to pack byte fields into one register.
fn generate_byte_packed_to_register(fields: &[FieldMapping]) -> TokenStream {
    let byte_packs = fields.iter().map(|mapping| {
        if let FieldMapping::BytePacked { field, is_high_byte } = mapping {
            let field_name = field.ident.as_ref().unwrap();
            if *is_high_byte {
                quote! {
                    register_value |= (self.#field_name as u16) << 8;
                }
            } else {
                quote! {
                    register_value |= self.#field_name as u16;
                }
            }
        } else {
            quote! {}
        }
    });

    quote! {
        {
            let mut register_value = 0u16;
            #(#byte_packs)*
            registers.push(register_value);
        }
    }
}

/// Generate the `FromRegisters` trait implementation.
fn generate_from_registers(fields: &[&ModbusFieldOpts], opts: &ModbusMapperOpts) -> TokenStream {
    let struct_name = &opts.ident;
    let default_endian = opts.get_default_endian();

    let groups = group_fields_by_address(fields, &default_endian);

    // Build a map from address to register index
    let mut address_to_index: BTreeMap<u16, usize> = BTreeMap::new();
    let mut current_index = 0usize;

    for group in &groups {
        address_to_index.insert(group.address, current_index);

        // Determine how many registers this group consumes
        let register_count = if group.fields.is_empty() {
            0
        } else {
            match &group.fields[0] {
                FieldMapping::Normal { register_count, .. } => *register_count as usize,
                FieldMapping::BitPacked { .. } | FieldMapping::BytePacked { .. } => 1,
            }
        };

        current_index += register_count;
    }

    // Generate field deserializations
    let field_deserializations = groups.iter().map(|group| {
        let index = address_to_index[&group.address];
        generate_register_group_from_registers(group, index)
    });

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
                    #(#field_deserializations)*
                })
            }
        }
    }
}

/// Generate code to deserialize a register group.
fn generate_register_group_from_registers(group: &RegisterGroup, index: usize) -> TokenStream {
    if group.fields.is_empty() {
        return quote! {};
    }

    let first = &group.fields[0];

    match first {
        FieldMapping::Normal { .. } => {
            generate_normal_field_from_registers(&group.fields[0], index)
        }
        FieldMapping::BitPacked { .. } => {
            let extractions = group.fields.iter().map(|mapping| {
                if let FieldMapping::BitPacked { field, bit_position } = mapping {
                    let field_name = field.ident.as_ref().unwrap();
                    quote! {
                        #field_name: (registers[#index] & (1u16 << #bit_position)) != 0,
                    }
                } else {
                    quote! {}
                }
            });
            quote! { #(#extractions)* }
        }
        FieldMapping::BytePacked { .. } => {
            let extractions = group.fields.iter().map(|mapping| {
                if let FieldMapping::BytePacked { field, is_high_byte } = mapping {
                    let field_name = field.ident.as_ref().unwrap();
                    let type_str = type_to_string(&field.ty);
                    let cast_type = if type_str == "i8" {
                        quote! { i8 }
                    } else {
                        quote! { u8 }
                    };

                    if *is_high_byte {
                        quote! {
                            #field_name: (registers[#index] >> 8) as #cast_type,
                        }
                    } else {
                        quote! {
                            #field_name: (registers[#index] & 0xFF) as #cast_type,
                        }
                    }
                } else {
                    quote! {}
                }
            });
            quote! { #(#extractions)* }
        }
    }
}

/// Generate code to deserialize a normal field.
fn generate_normal_field_from_registers(mapping: &FieldMapping, index: usize) -> TokenStream {
    let FieldMapping::Normal { field, endian, type_str, .. } = mapping else {
        return quote! {};
    };

    let field_name = field.ident.as_ref().unwrap();

    match type_str.as_str() {
        "u8" => {
            quote! {
                #field_name: registers[#index] as u8,
            }
        }
        "i8" => {
            quote! {
                #field_name: registers[#index] as i8,
            }
        }
        "u16" => {
            quote! {
                #field_name: registers[#index],
            }
        }
        "i16" => {
            quote! {
                #field_name: registers[#index] as i16,
            }
        }
        "u32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::u32_from_registers(
                    &registers[#index..#index + 2],
                    #endian_enum
                ),
            }
        }
        "i32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::i32_from_registers(
                    &registers[#index..#index + 2],
                    #endian_enum
                ),
            }
        }
        "u64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::u64_from_registers(
                    &registers[#index..#index + 4],
                    #endian_enum
                ),
            }
        }
        "i64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::i64_from_registers(
                    &registers[#index..#index + 4],
                    #endian_enum
                ),
            }
        }
        "f32" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::f32_from_registers(
                    &registers[#index..#index + 2],
                    #endian_enum
                ),
            }
        }
        "f64" => {
            let endian_enum = endian_to_enum(endian);
            quote! {
                #field_name: ::modbus_mapper::endian::f64_from_registers(
                    &registers[#index..#index + 4],
                    #endian_enum
                ),
            }
        }
        "bool" => {
            quote! {
                #field_name: registers[#index] != 0,
            }
        }
        _ => {
            let field_type = &field.ty;
            quote! {
                #field_name: compile_error!(concat!("Unsupported type: ", stringify!(#field_type))),
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

        // For metadata, we report the field's individual contribution
        let register_count = if field.bit.is_some() || field.offset.is_some() {
            // Packed fields share a register, but for metadata we report 1
            1
        } else {
            let type_str = type_to_string(&field.ty);
            get_type_register_count(&type_str)
        };

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

/// Calculate the total register count from register groups.
fn calculate_total_register_count_from_groups(groups: &[RegisterGroup]) -> u16 {
    groups
        .iter()
        .map(|group| {
            if group.fields.is_empty() {
                0
            } else {
                match &group.fields[0] {
                    FieldMapping::Normal { register_count, .. } => *register_count,
                    FieldMapping::BitPacked { .. } | FieldMapping::BytePacked { .. } => 1,
                }
            }
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
