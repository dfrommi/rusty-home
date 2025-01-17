use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_id_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let enum_variants = super::enum_variants(input.data);

    let type_name_int = enum_name.to_string();
    let type_name_ext = enum_name.to_string().to_snake_case();

    let mut into_int_id_impls = Vec::new();
    let mut into_ext_id_impls = Vec::new();
    let mut from_ext_item_name_matches = Vec::new();
    let mut from_int_item_name_matches = Vec::new();
    let mut display_impls = Vec::new();

    for variant in enum_variants {
        let variant_name = &variant.ident;
        let variant_name_int = variant_name.to_string();
        let variant_name_ext = variant_name.to_string().to_snake_case();

        into_int_id_impls.push(quote! {
            #enum_name::#variant_name => support::InternalId::new(#type_name_int, #variant_name_int)
        });

        into_ext_id_impls.push(quote! {
            #enum_name::#variant_name => support::ExternalId::new(#type_name_ext, #variant_name_ext)
        });

        from_int_item_name_matches.push(quote! {
            #variant_name_int => #enum_name::#variant_name
        });

        from_ext_item_name_matches.push(quote! {
            #variant_name_ext => #enum_name::#variant_name
        });

        //not snake cased
        let display_name = format!("{}[{}]", enum_name, variant_name);
        display_impls.push(quote! {
            #enum_name::#variant_name => write!(f, #display_name)
        });
    }

    let expanded = quote! {
        impl From<&#enum_name> for support::InternalId {
            fn from(val: &#enum_name) -> Self {
                match val {
                    #(#into_int_id_impls),*
                }
            }
        }

        impl From<#enum_name> for support::InternalId {
            fn from(val: #enum_name) -> Self {
                (&val).into()
            }
        }

        impl From<&#enum_name> for support::ExternalId {
            fn from(val: &#enum_name) -> Self {
                match val {
                    #(#into_ext_id_impls),*
                }
            }
        }

        impl From<#enum_name> for support::ExternalId {
            fn from(val: #enum_name) -> Self {
                (&val).into()
            }
        }

        impl TryFrom<&support::InternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: &support::InternalId) -> Result<Self, Self::Error> {
                if value.type_ != #type_name_int {
                    anyhow::bail!("Error converting InternalId, expected type {}, got {}", #type_name_int, value.type_);
                }

                let item = match value.name.as_str() {
                    #(#from_int_item_name_matches),*,
                    _ => anyhow::bail!("Error converting InternalId, unknown name {}", value.name),
                };

                Ok(item)
            }
        }

        impl TryFrom<support::InternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: support::InternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl TryFrom<&support::ExternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: &support::ExternalId) -> Result<Self, Self::Error> {
                if value.type_ != #type_name_ext {
                    anyhow::bail!("Error converting ExternalId, expected type {}, got {}", #type_name_ext, value.type_);
                }

                let item = match value.name.as_str() {
                    #(#from_ext_item_name_matches),*,
                    _ => anyhow::bail!("Error converting ExternalId, unknown name {}", value.name),
                };

                Ok(item)
            }
        }

        impl TryFrom<support::ExternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: support::ExternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let id = support::InternalId::from(self);
                write!(f, "{}[{}]", id.type_, id.name)
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn derive_id_item_delegation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., ChannelValue
    let name = input.ident;

    // Ensure it's an enum
    let variants = super::enum_variants(input.data);

    let mut value_into_impls = Vec::new();
    let mut try_from_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        value_into_impls.push(quote! {
            #name::#variant_name(v) => v.into()
        });

        try_from_impls.push(quote! {
            if let Ok(item) = #name::try_from(value) {
                return Ok(item);
            }
        });
    }

    let expanded = quote! {

        impl From<#name> for support::InternalId {
            fn from(val: #name) -> Self {
                match val {
                    #(#value_into_impls),*
                }
            }
        }

        impl From<&#name> for support::InternalId {
            fn from(val: &#name) -> Self {
                match val {
                    #(#value_into_impls),*
                }
            }
        }

        impl From<#name> for support::ExternalId {
            fn from(val: #name) -> Self {
                match val {
                    #(#value_into_impls),*
                }
            }
        }

        impl From<&#name> for support::ExternalId {
            fn from(val: &#name) -> Self {
                match val {
                    #(#value_into_impls),*
                }
            }
        }

        impl TryFrom<&support::InternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: &support::InternalId) -> Result<Self, Self::Error> {
                #(#try_from_impls)*
                anyhow::bail!("Error converting InternalId, unknown type/name {}/{}", value.type_, value.name);
            }
        }

        impl TryFrom<support::InternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: support::InternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl TryFrom<&support::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: &support::ExternalId) -> Result<Self, Self::Error> {
                #(#try_from_impls)*
                anyhow::bail!("Error converting ExternalId, unknown type/name {}/{}", value.type_, value.name);
            }
        }

        impl TryFrom<support::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: support::ExternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }
    };

    TokenStream::from(expanded)
}
