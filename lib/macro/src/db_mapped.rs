use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn db_mapped(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let variants = super::enum_variants(input.data);

    let mut type_info_impls = Vec::new();
    let mut into_dbvalue_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        if let syn::Fields::Unnamed(fields) = variant.fields {
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            // Generate the ChannelTypeInfo implementation
            type_info_impls.push(quote! {
                impl crate::core::ValueObject for #item_type {
                    type ValueType = #value_type;
                }
            });

            // Generate the Into<f64> implementation
            into_dbvalue_impls.push(quote! {
                #name::#variant_name(_, v) => v.into()
            });
        }
    }

    let expanded = quote! {
        // Implement ChannelTypeInfo for each variant
        #(#type_info_impls)*

        impl From<&#name> for crate::core::persistence::DbValue {
            fn from(val: &#name) -> Self {
                match val {
                    #(#into_dbvalue_impls),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
