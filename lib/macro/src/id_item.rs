use heck::ToShoutySnakeCase;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_id_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let enum_variants = super::enum_variants(input.data);

    let type_name_int = enum_name.to_string();
    let type_name_ext = enum_name.to_string().to_snake_case();

    let mut as_ref_int_statics = Vec::new();
    let mut as_ref_int_matches = Vec::new();
    let mut as_ref_ext_statics = Vec::new();
    let mut as_ref_ext_matches = Vec::new();

    let mut from_ext_item_name_matches = Vec::new();
    let mut from_int_item_name_matches = Vec::new();
    let mut display_impls = Vec::new();

    for variant in enum_variants {
        let variant_name = &variant.ident;
        let id_static_name = Ident::new(
            &format!("{}_ID", variant_name.to_string().to_shouty_snake_case()),
            variant_name.span(),
        );

        let variant_name_int = variant_name.to_string();
        let variant_name_ext = variant_name.to_string().to_snake_case();

        as_ref_int_statics.push(quote! {
            static #id_static_name: crate::core::id::InternalId = crate::core::id::InternalId::new(#type_name_int, #variant_name_int);
        });
        as_ref_int_matches.push(quote! {
            #enum_name::#variant_name => &#id_static_name
        });

        as_ref_ext_statics.push(quote! {
            static #id_static_name: crate::core::id::ExternalId = crate::core::id::ExternalId::new_static(#type_name_ext, #variant_name_ext);
        });
        as_ref_ext_matches.push(quote! {
            #enum_name::#variant_name => &#id_static_name
        });

        from_int_item_name_matches.push(quote! {
            #variant_name_int => #enum_name::#variant_name
        });

        from_ext_item_name_matches.push(quote! {
            #variant_name_ext => #enum_name::#variant_name
        });

        let display_name = format!("{}[{}]", enum_name, variant_name);
        display_impls.push(quote! {
            #enum_name::#variant_name => write!(f, #display_name)
        });
    }

    let expanded = quote! {
        impl AsRef<crate::core::id::InternalId> for #enum_name {
            fn as_ref(&self) -> &crate::core::id::InternalId {
                #(#as_ref_int_statics)*

                match self {
                    #(#as_ref_int_matches),*
                }
            }
        }

        impl AsRef<crate::core::id::ExternalId> for #enum_name {
            fn as_ref(&self) -> &crate::core::id::ExternalId {
                #(#as_ref_ext_statics)*

                match self {
                    #(#as_ref_ext_matches),*
                }
            }
        }

        impl #enum_name {
            pub fn int_type(&self) -> &str {
                let id: &crate::core::id::InternalId = self.as_ref();
                id.int_type()
            }

            pub fn int_name(&self) -> &str {
                let id: &crate::core::id::InternalId = self.as_ref();
                id.int_name()
            }

            pub fn ext_type(&self) -> &str {
                let id: &crate::core::id::ExternalId = self.as_ref();
                id.ext_type()
            }

            pub fn ext_name(&self) -> &str {
                let id: &crate::core::id::ExternalId = self.as_ref();
                id.ext_name()
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

    let mut value_as_ref_impls = Vec::new();
    let mut try_from_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        value_as_ref_impls.push(quote! {
            #name::#variant_name(v) => v.as_ref()
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
        impl AsRef<crate::core::id::InternalId> for #name {
            fn as_ref(&self) -> &crate::core::id::InternalId {
                match self {
                    #(#value_as_ref_impls),*
                }
            }
        }

        impl AsRef<crate::core::id::ExternalId> for #name {
            fn as_ref(&self) -> &crate::core::id::ExternalId {
                match self {
                    #(#value_as_ref_impls),*
                }
            }
        }

        impl #name {
            pub fn int_type(&self) -> &str {
                let id: &crate::core::id::InternalId = self.as_ref();
                id.int_type()
            }

            pub fn int_name(&self) -> &str {
                let id: &crate::core::id::InternalId = self.as_ref();
                id.int_name()
            }

            pub fn ext_type(&self) -> &str {
                let id: &crate::core::id::ExternalId = self.as_ref();
                id.ext_type()
            }

            pub fn ext_name(&self) -> &str {
                let id: &crate::core::id::ExternalId = self.as_ref();
                id.ext_name()
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
