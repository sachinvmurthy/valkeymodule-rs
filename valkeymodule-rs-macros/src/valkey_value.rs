use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use serde::Deserialize;
use serde_syn::{config, from_stream};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields,
};

/// Generate [From] implementation for [ValkeyValue] for Enum.
/// The generated code will simply check the Enum current type (using
/// a match statement) and will perform [Into] and the matched variant.
fn enum_valkey_value(struct_name: Ident, enum_data: DataEnum) -> TokenStream {
    let variants = enum_data
        .variants
        .into_iter()
        .map(|v| v.ident)
        .collect::<Vec<_>>();

    let res = quote! {
        impl From<#struct_name> for valkey_module::redisvalue::ValkeyValue {
            fn from(val: #struct_name) -> valkey_module::redisvalue::ValkeyValue {
                match val {
                    #(
                        #struct_name::#variants(v) => v.into(),
                    )*
                }
            }
        }
    };
    res.into()
}

/// Represent a single field attributes
#[derive(Debug, Deserialize, Default)]
struct FieldAttr {
    flatten: bool,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        from_stream(config::JSONY, input)
    }
}

/// Generate [From] implementation for [ValkeyValue] for a struct.
/// The generated code will create a [ValkeyValue::Map] element such that
/// the keys are the fields names and the value are the result of
/// running [Into] on each field value to convert it to [ValkeyValue].
fn struct_valkey_value(struct_name: Ident, struct_data: DataStruct) -> TokenStream {
    let fields = match struct_data.fields {
        Fields::Named(f) => f,
        _ => {
            return quote! {compile_error!("ValkeyValue derive can only be apply on struct with named fields.")}.into()
        }
    };

    let fields = fields
        .named
        .into_iter()
        .map(|v| {
            let name = v
                .ident
                .ok_or("Field without a name is not supported.".to_owned())?;
            if v.attrs.len() > 1 {
                return Err("Expected at most a single attribute for each field".to_owned());
            }
            let field_attr = v.attrs.into_iter().next().map_or(
                Ok::<_, String>(FieldAttr::default()),
                |attr| {
                    let tokens = attr.tokens;
                    let field_attr: FieldAttr =
                        parse_macro_input::parse(tokens.into()).map_err(|e| format!("{e}"))?;
                    Ok(field_attr)
                },
            )?;
            Ok((name, field_attr))
        })
        .collect::<Result<Vec<_>, String>>();

    let fields = match fields {
        Ok(f) => f,
        Err(e) => return quote! {compile_error!(#e)}.into(),
    };

    let (fields, flattem_fields) = fields.into_iter().fold(
        (Vec::new(), Vec::new()),
        |(mut fields, mut flatten_fields), (field, attr)| {
            if attr.flatten {
                flatten_fields.push(field);
            } else {
                fields.push(field);
            }

            (fields, flatten_fields)
        },
    );

    let fields_names: Vec<_> = fields.iter().map(|v| v.to_string()).collect();

    let res = quote! {
        impl From<#struct_name> for valkey_module::redisvalue::ValkeyValue {
            fn from(val: #struct_name) -> valkey_module::redisvalue::ValkeyValue {
                let mut fields: std::collections::BTreeMap<valkey_module::redisvalue::ValkeyValueKey, valkey_module::redisvalue::ValkeyValue> = std::collections::BTreeMap::from([
                    #((
                        valkey_module::redisvalue::ValkeyValueKey::String(#fields_names.to_owned()),
                        val.#fields.into()
                    ), )*
                ]);
                #(
                    let flatten_field: std::collections::BTreeMap<valkey_module::redisvalue::ValkeyValueKey, valkey_module::redisvalue::ValkeyValue> = val.#flattem_fields.into();
                    fields.extend(flatten_field.into_iter());
                )*
                valkey_module::redisvalue::ValkeyValue::OrderedMap(fields)
            }
        }

        impl From<#struct_name> for std::collections::BTreeMap<valkey_module::redisvalue::ValkeyValueKey, valkey_module::redisvalue::ValkeyValue> {
            fn from(val: #struct_name) -> std::collections::BTreeMap<valkey_module::redisvalue::ValkeyValueKey, valkey_module::redisvalue::ValkeyValue> {
                std::collections::BTreeMap::from([
                    #((
                        valkey_module::redisvalue::ValkeyValueKey::String(#fields_names.to_owned()),
                        val.#fields.into()
                    ), )*
                ])
            }
        }
    };
    res.into()
}

/// Implementation for [ValkeyValue] derive proc macro.
/// Runs the relevant code generation base on the element
/// the proc macro was used on. Currently supports Enums and
/// structs.
pub fn valkey_value(item: TokenStream) -> TokenStream {
    let struct_input: DeriveInput = parse_macro_input!(item);
    let struct_name = struct_input.ident;
    match struct_input.data {
        Data::Struct(s) => struct_valkey_value(struct_name, s),
        Data::Enum(e) => enum_valkey_value(struct_name, e),
        _ => quote! {compile_error!("ValkeyValue derive can only be apply on struct.")}.into(),
    }
}
