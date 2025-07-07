use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., ChannelValue
    let name = input.ident;
    let target_enum_name = format_ident!(
        "{}",
        name.to_string()
            .strip_suffix("Value")
            .expect("Expected input enum to end with Value")
    );
    let variants = super::enum_variants(input.data);

    // Collect the Channel enum variants and the implementations
    let mut target_variants = Vec::new();
    let mut item_to_target_impls = Vec::new();
    let mut source_to_target_impl = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        if let syn::Fields::Unnamed(fields) = variant.fields {
            let item_type = &fields.unnamed[0].ty;

            // Generate the variant for the Channel enum
            target_variants.push(quote! {
                #variant_name(#item_type)
            });

            // Generate the From implementation
            item_to_target_impls.push(quote! {
                impl From<#item_type> for #target_enum_name {
                    fn from(val: #item_type) -> Self {
                        #target_enum_name::#variant_name(val)
                    }
                }

                impl From<&#item_type> for #target_enum_name {
                    fn from(val: &#item_type) -> Self {
                        #target_enum_name::#variant_name(val.clone())
                    }
                }
            });

            // Generate the From<&ChannelValue> for Channel implementation
            source_to_target_impl.push(quote! {
                #name::#variant_name(id, _) => #target_enum_name::#variant_name(id.clone())
            });
        }
    }

    let expanded = quote! {
        // Define the Channel enum
        #[derive(Debug, Clone, Hash, Eq, PartialEq, r#macro::IdDelegation)]
        pub enum #target_enum_name {
            #(#target_variants),*
        }

        // Implement From for each variant
        #(#item_to_target_impls)*

        // Implement From<&ChannelValue> for Channel
        impl From<&#name> for #target_enum_name {
            fn from(val: &#name) -> Self {
                match val {
                    #(#source_to_target_impl),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
