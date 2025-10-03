use heck::ToShoutySnakeCase;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_id_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let enum_variants = super::enum_variants(input.data);

    let type_name_int = enum_name.to_string();
    let type_name_ext = enum_name.to_string().to_snake_case();

    let mut int_id_statics = Vec::new();
    let mut int_id_matches = Vec::new();
    let mut ext_id_statics = Vec::new();
    let mut ext_id_matches = Vec::new();

    let mut from_ext_item_name_matches = Vec::new();
    let mut from_int_item_name_matches = Vec::new();
    let mut display_impls = Vec::new();

    for variant in enum_variants {
        let variant_name = &variant.ident;
        let static_suffix = variant_name.to_string().to_shouty_snake_case();
        let int_id_static_name = Ident::new(&format!("{}_INT_ID", static_suffix), variant_name.span());
        let ext_id_static_name = Ident::new(&format!("{}_EXT_ID", static_suffix), variant_name.span());

        let variant_name_int = variant_name.to_string();
        let variant_name_ext = variant_name.to_string().to_snake_case();

        int_id_statics.push(quote! {
            static #int_id_static_name: crate::core::id::InternalId = crate::core::id::InternalId::new(#type_name_int, #variant_name_int);
        });
        int_id_matches.push(quote! {
            #enum_name::#variant_name => &#int_id_static_name
        });

        ext_id_statics.push(quote! {
            static #ext_id_static_name: crate::core::id::ExternalId = crate::core::id::ExternalId::new_static(#type_name_ext, #variant_name_ext);
        });
        ext_id_matches.push(quote! {
            #enum_name::#variant_name => &#ext_id_static_name
        });

        from_int_item_name_matches.push(quote! {
            #variant_name_int => #enum_name::#variant_name
        });

        from_ext_item_name_matches.push(quote! {
            #variant_name_ext => #enum_name::#variant_name
        });

        let display_name = format!("{enum_name}[{variant_name}]");
        display_impls.push(quote! {
            #enum_name::#variant_name => write!(f, #display_name)
        });
    }

    let expanded = quote! {
        impl #enum_name {
            pub fn int_id(&self) -> &'static crate::core::id::InternalId {
                #(#int_id_statics)*

                match self {
                    #(#int_id_matches),*
                }
            }

            pub fn ext_id(&self) -> &'static crate::core::id::ExternalId {
                #(#ext_id_statics)*

                match self {
                    #(#ext_id_matches),*
                }
            }

            pub fn int_type(&self) -> &str {
                self.int_id().int_type()
            }

            pub fn int_name(&self) -> &str {
                self.int_id().int_name()
            }

            pub fn ext_type(&self) -> &str {
                self.ext_id().ext_type()
            }

            pub fn ext_name(&self) -> &str {
                self.ext_id().ext_name()
            }
        }

        impl TryFrom<&crate::core::id::InternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: &crate::core::id::InternalId) -> Result<Self, Self::Error> {
                if value.int_type() != #type_name_int {
                    anyhow::bail!("Error converting InternalId, expected type {}, got {}", #type_name_int, value.int_type());
                }

                let item = match value.int_name() {
                    #(#from_int_item_name_matches),*,
                    _ => anyhow::bail!("Error converting InternalId, unknown name {}", value.int_name()),
                };

                Ok(item)
            }
        }

        impl TryFrom<crate::core::id::InternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: crate::core::id::InternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl TryFrom<&crate::core::id::ExternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: &crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                if value.ext_type() != #type_name_ext {
                    anyhow::bail!("Error converting ExternalId, expected type {}, got {}", #type_name_ext, value.ext_type());
                }

                let item = match value.ext_name() {
                    #(#from_ext_item_name_matches),*,
                    _ => anyhow::bail!("Error converting ExternalId, unknown name {}", value.ext_name()),
                };

                Ok(item)
            }
        }

        impl TryFrom<crate::core::id::ExternalId> for #enum_name {
            type Error = anyhow::Error;

            fn try_from(value: crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}[{}]", self.int_type(), self.int_name())
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

    let mut int_id_matches = Vec::new();
    let mut ext_id_matches = Vec::new();
    let mut try_from_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        int_id_matches.push(quote! {
            #name::#variant_name(v) => v.int_id()
        });

        ext_id_matches.push(quote! {
            #name::#variant_name(v) => v.ext_id()
        });

        //should always be true
        if let syn::Fields::Unnamed(fields) = variant.fields {
            let item_type = &fields.unnamed[0].ty;

            try_from_impls.push(quote! {
                if let Ok(item) = #item_type::try_from(value) {
                    return Ok(#name::#variant_name(item));
                }
            });
        }
    }

    let expanded = quote! {
        impl #name {
            pub fn int_id(&self) -> &'static crate::core::id::InternalId {
                match self {
                    #(#int_id_matches),*
                }
            }

            pub fn ext_id(&self) -> &'static crate::core::id::ExternalId {
                match self {
                    #(#ext_id_matches),*
                }
            }

            pub fn int_type(&self) -> &str {
                self.int_id().int_type()
            }

            pub fn int_name(&self) -> &str {
                self.int_id().int_name()
            }

            pub fn ext_type(&self) -> &str {
                self.ext_id().ext_type()
            }

            pub fn ext_name(&self) -> &str {
                self.ext_id().ext_name()
            }
        }

        impl TryFrom<&crate::core::id::InternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: &crate::core::id::InternalId) -> Result<Self, Self::Error> {
                #(#try_from_impls)*
                anyhow::bail!("Error converting InternalId, unknown type/name {}/{}", value.int_type(), value.int_name());
            }
        }

        impl TryFrom<crate::core::id::InternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: crate::core::id::InternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }

        impl TryFrom<&crate::core::id::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: &crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                #(#try_from_impls)*
                anyhow::bail!("Error converting ExternalId, unknown type/name {}/{}", value.ext_type(), value.ext_name());
            }
        }

        impl TryFrom<crate::core::id::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }
    };

    TokenStream::from(expanded)
}
