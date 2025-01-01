use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

pub fn derive_typed_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let enum_variants = super::enum_variants(input.data);

    // Generate type_name and item_name implementations
    let type_name = enum_name.to_string().to_snake_case();
    let variants = generate_variants_snake_case(&enum_variants);

    let from_item_name_matches = generate_from_item_name_matches(&enum_name, &enum_variants);

    let expanded = quote! {
        impl support::TypedItem for #enum_name {
            fn type_name(&self) -> &'static str {
                #type_name
            }

            fn item_name(&self) -> &'static str {
                match self {
                    #(#variants)*
                }
            }
        }

        impl #enum_name {
            pub const TYPE_NAME: &'static str = #type_name;

            pub fn from_item_name(name: &str) -> Option<Self> {
                match name {
                    #(#from_item_name_matches)*
                    _ => None,
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

fn generate_variants_snake_case(enum_variants: &[syn::Variant]) -> Vec<proc_macro2::TokenStream> {
    enum_variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let item_name = variant_name.to_string().to_snake_case();
            quote! {
                Self::#variant_name => #item_name,
            }
        })
        .collect()
}

fn generate_from_item_name_matches(
    enum_name: &Ident,
    enum_variants: &[syn::Variant],
) -> Vec<proc_macro2::TokenStream> {
    enum_variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let item_name = variant_name.to_string().to_snake_case();
            quote! {
                #item_name => Some(#enum_name::#variant_name),
            }
        })
        .collect()
}
