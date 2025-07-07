use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn db_mapped(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let variants = super::enum_variants(input.data);

    let mut type_info_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        if let syn::Fields::Unnamed(fields) = variant.fields {
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            let is_bool = match &value_type {
                syn::Type::Path(type_path) => type_path.path.segments.last().unwrap().ident == "bool",
                _ => false,
            };

            let impl_block = if is_bool {
                quote! {
                    impl crate::core::ValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(value: &#value_type) -> f64 {
                            if *value { 1.0 } else { 0.0 }
                        }

                        fn from_f64(value: f64) -> #value_type {
                            value > 0.0
                        }
                    }
                }
            } else {
                quote! {
                    impl crate::core::ValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(value: &#value_type) -> f64 {
                            value.into()
                        }

                        fn from_f64(value: f64) -> #value_type {
                            value.into()
                        }
                    }
                }
            };
            type_info_impls.push(impl_block);
        }
    }

    let expanded = quote! {
        // Implement ChannelTypeInfo for each variant
        #(#type_info_impls)*
    };

    TokenStream::from(expanded)
}
