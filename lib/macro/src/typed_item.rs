use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_typed_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let enum_variants = super::enum_variants(input.data);

    let type_name_snake = enum_name.to_string().to_snake_case();

    let mut item_name_impls = Vec::new();
    let mut from_item_name_matches = Vec::new();
    let mut display_impls = Vec::new();

    for variant in enum_variants {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_snake_case();

        item_name_impls.push(quote! {
            #enum_name::#variant_name => #variant_name_snake
        });

        from_item_name_matches.push(quote! {
            #variant_name_snake => Some(#enum_name::#variant_name)
        });

        //not snake cased
        let display_name = format!("{}[{}]", enum_name, variant_name);
        display_impls.push(quote! {
            #enum_name::#variant_name => write!(f, #display_name)
        });
    }

    let expanded = quote! {
        impl support::TypedItem for #enum_name {
            fn type_name(&self) -> &'static str {
                #type_name_snake
            }

            fn item_name(&self) -> &'static str {
                match self {
                    #(#item_name_impls),*
                }
            }
        }

        impl #enum_name {
            pub const TYPE_NAME: &'static str = #type_name_snake;

            pub fn from_item_name(name: &str) -> Option<Self> {
                match name {
                    #(#from_item_name_matches),*,
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_impls),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn derive_typed_item_delegation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., ChannelValue
    let name = input.ident;

    // Ensure it's an enum
    let variants = super::enum_variants(input.data);

    let mut typed_item_type_impls = Vec::new();
    let mut typed_item_item_impls = Vec::new();
    let mut typed_item_from_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_snake_case();

        typed_item_type_impls.push(quote! {
            #name::#variant_name(v) => v.type_name()
        });

        typed_item_item_impls.push(quote! {
            #name::#variant_name(v) => v.item_name()
        });

        typed_item_from_impls.push(quote! {
            #variant_name_snake => {
                #variant_name::from_item_name(item_name).map(#name::#variant_name)
            }
        });
    }

    let expanded = quote! {

        impl support::TypedItem for #name {
            fn type_name(&self) -> &'static str {
                match self {
                    #(#typed_item_type_impls),*
                }
            }

            fn item_name(&self) -> &'static str {
                match self {
                    #(#typed_item_item_impls),*
                }
            }
        }

        impl #name {
            pub fn from_type_and_item(type_name: &str, item_name: &str) -> Option<Self> {
                match type_name {
                    #(#typed_item_from_impls),*
                    _ => None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
