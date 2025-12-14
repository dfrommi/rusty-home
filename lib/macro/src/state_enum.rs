use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{DeriveInput, Fields, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;
    let variants = super::enum_variants(input.data);

    let base_name = enum_name
        .to_string()
        .strip_suffix("Value")
        .expect("StateEnumDerive expects enum names to end with Value")
        .to_string();
    let id_enum_name = format_ident!("{}Id", base_name);
    let item_trait_name = format_ident!("{}Item", base_name);

    let id_enum = generate_id_enum(&enum_name, &id_enum_name, &variants);
    let item_trait = generate_item_trait(&enum_name, &item_trait_name);
    let item_impls = generate_item_impls(&enum_name, &item_trait_name, &variants, &base_name);

    TokenStream::from(quote! {
        #item_trait
        #id_enum
        #(#item_impls)*
    })
}

fn generate_id_enum(enum_name: &syn::Ident, id_enum_name: &syn::Ident, variants: &[syn::Variant]) -> TokenStream2 {
    let mut id_variants = Vec::new();
    let mut item_to_id_impls = Vec::new();

    let from_ref_matches = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! { #enum_name::#variant_name(id, _) => #id_enum_name::#variant_name(id.clone()) }
    });

    for variant in variants {
        let variant_name = &variant.ident;

        if let Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() != 2 {
                panic!("StateEnumDerive expects tuple variants with exactly two fields");
            }

            let id_type = &fields.unnamed[0].ty;

            id_variants.push(quote! { #variant_name(#id_type) });

            item_to_id_impls.push(quote! {
                impl From<#id_type> for #id_enum_name {
                    fn from(id: #id_type) -> Self {
                        #id_enum_name::#variant_name(id)
                    }
                }

                impl From<&#id_type> for #id_enum_name {
                    fn from(id: &#id_type) -> Self {
                        #id_enum_name::#variant_name(id.clone())
                    }
                }
            });
        } else {
            panic!("StateEnumDerive expects tuple variants");
        }
    }

    let from_matches = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! { #enum_name::#variant_name(id, _) => #id_enum_name::#variant_name(id) }
    });

    quote! {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, r#macro::IdDelegation, r#macro::EnumVariants)]
        pub enum #id_enum_name {
            #(#id_variants),*
        }

        impl From<#enum_name> for #id_enum_name {
            fn from(value: #enum_name) -> Self {
                match value {
                    #(#from_matches),*
                }
            }
        }

        impl From<&#enum_name> for #id_enum_name {
            fn from(value: &#enum_name) -> Self {
                match value {
                    #(#from_ref_matches),*
                }
            }
        }

        #(#item_to_id_impls)*
    }
}

fn generate_item_trait(enum_name: &syn::Ident, item_trait_name: &syn::Ident) -> TokenStream2 {
    quote! {
        pub trait #item_trait_name {
            type Type: Clone;
            fn try_downcast(&self, value: #enum_name) -> anyhow::Result<Self::Type>;
        }
    }
}

fn generate_item_impls(
    enum_name: &syn::Ident,
    item_trait_name: &syn::Ident,
    variants: &[syn::Variant],
    base_name: &str,
) -> Vec<TokenStream2> {
    let error_prefix = base_name.to_snake_case().replace('_', " ");
    let error_message = format!("Unexpected {} type for {{:?}}: {{:?}}", error_prefix);

    variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;

            if let Fields::Unnamed(fields) = &variant.fields {
                if fields.unnamed.len() != 2 {
                    panic!("StateEnumDerive expects tuple variants with exactly two fields");
                }

                let item_type = &fields.unnamed[0].ty;
                let value_type = &fields.unnamed[1].ty;

                quote! {
                    impl #item_trait_name for #item_type {
                        type Type = #value_type;

                        fn try_downcast(&self, value: #enum_name) -> anyhow::Result<Self::Type> {
                            match value {
                                #enum_name::#variant_name(_, v) => Ok(v),
                                _ => Err(anyhow::anyhow!(#error_message, self, value)),
                            }
                        }
                    }
                }
            } else {
                panic!("StateEnumDerive expects tuple variants");
            }
        })
        .collect()
}
