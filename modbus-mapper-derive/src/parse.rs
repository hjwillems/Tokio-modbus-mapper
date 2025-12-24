//! Attribute parsing for ModbusMapper derive macro.

use darling::{ast, FromDeriveInput, FromField};
use syn::{Ident, Type};

/// Struct-level attributes for `#[modbus(...)]`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(modbus), supports(struct_named))]
pub struct ModbusMapperOpts {
    /// The struct identifier.
    pub ident: Ident,

    /// The struct's fields.
    pub data: ast::Data<(), ModbusFieldOpts>,

    /// Base address for all fields (optional, defaults to 0).
    #[darling(default)]
    pub base_address: Option<u16>,

    /// Register type: "holding", "input", "coil", or "discrete".
    #[darling(default)]
    pub register_type: Option<String>,

    /// Default endianness for multi-register types: "big" or "little".
    #[darling(default)]
    pub default_endian: Option<String>,
}

/// Field-level attributes for `#[modbus(...)]`.
#[derive(Debug, FromField)]
#[darling(attributes(modbus))]
pub struct ModbusFieldOpts {
    /// The field identifier.
    pub ident: Option<Ident>,

    /// The field type.
    pub ty: Type,

    /// Register address for this field (required unless skip is true).
    #[darling(default)]
    pub address: Option<u16>,

    /// Endianness for this field: "big" or "little" (optional).
    #[darling(default)]
    pub endian: Option<String>,

    /// Skip this field in Modbus mapping.
    #[darling(default)]
    pub skip: bool,

    /// Read-only field (server mode).
    #[darling(default)]
    pub readonly: bool,

    /// Write-only field (server mode).
    #[darling(default)]
    pub writeonly: bool,
}

impl ModbusMapperOpts {
    /// Get the register type, defaulting to "holding" if not specified.
    pub fn get_register_type(&self) -> String {
        self.register_type
            .clone()
            .unwrap_or_else(|| "holding".to_string())
    }

    /// Get the default endianness, defaulting to "big" if not specified.
    pub fn get_default_endian(&self) -> String {
        self.default_endian
            .clone()
            .unwrap_or_else(|| "big".to_string())
    }

    /// Get the base address, defaulting to 0 if not specified.
    pub fn get_base_address(&self) -> u16 {
        self.base_address.unwrap_or(0)
    }

    /// Get all non-skipped fields.
    pub fn fields(&self) -> Vec<&ModbusFieldOpts> {
        match &self.data {
            ast::Data::Struct(fields) => fields
                .iter()
                .filter(|f| !f.skip)
                .collect(),
            _ => vec![],
        }
    }
}

impl ModbusFieldOpts {
    /// Get the field name as a string.
    pub fn name(&self) -> String {
        self.ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Get the endianness for this field, using the provided default if not specified.
    pub fn get_endian(&self, default: &str) -> String {
        self.endian.clone().unwrap_or_else(|| default.to_string())
    }
}

/// Validate struct-level attributes.
pub fn validate_struct_attrs(opts: &ModbusMapperOpts) -> Result<(), darling::Error> {
    // Validate register type
    let register_type = opts.get_register_type();
    if !matches!(
        register_type.as_str(),
        "holding" | "input" | "coil" | "discrete"
    ) {
        return Err(darling::Error::custom(format!(
            "Invalid register_type '{}'. Must be 'holding', 'input', 'coil', or 'discrete'",
            register_type
        ))
        .with_span(&opts.ident));
    }

    // Validate default endianness
    let default_endian = opts.get_default_endian();
    if !matches!(default_endian.as_str(), "big" | "little") {
        return Err(darling::Error::custom(format!(
            "Invalid default_endian '{}'. Must be 'big' or 'little'",
            default_endian
        ))
        .with_span(&opts.ident));
    }

    Ok(())
}

/// Validate field-level attributes.
pub fn validate_field_attrs(
    field: &ModbusFieldOpts,
    _default_endian: &str,
) -> Result<(), darling::Error> {
    // Skip validation for skipped fields
    if field.skip {
        return Ok(());
    }

    // Address is required for non-skipped fields
    if field.address.is_none() {
        return Err(darling::Error::custom(format!(
            "Field '{}' must have an 'address' attribute or be marked with 'skip'",
            field.name()
        ))
        .with_span(field.ident.as_ref().unwrap()));
    }

    // Validate endianness if specified
    if let Some(ref endian) = field.endian {
        if !matches!(endian.as_str(), "big" | "little") {
            return Err(darling::Error::custom(format!(
                "Invalid endian '{}' for field '{}'. Must be 'big' or 'little'",
                endian,
                field.name()
            ))
            .with_span(field.ident.as_ref().unwrap()));
        }
    }

    // Validate readonly/writeonly combination
    if field.readonly && field.writeonly {
        return Err(darling::Error::custom(format!(
            "Field '{}' cannot be both 'readonly' and 'writeonly'",
            field.name()
        ))
        .with_span(field.ident.as_ref().unwrap()));
    }

    Ok(())
}
