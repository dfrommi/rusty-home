use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;
    let variants = super::enum_variants(input.data);

    let mut type_info_impls = Vec::new();
    let mut persistent_variants = Vec::new();
    let mut persistent_enum_variants = Vec::new();
    let mut persistent_conversion_matches = Vec::new();
    let mut persistent_state_conversion_matches = Vec::new();

    // Check if this is HomeStateValue to generate persistent types
    let persistent_enum_name = format_ident!("Persistent{}", enum_name);

    // Generate HomeState name (strip "Value" suffix if present)
    let home_state_name = if enum_name.to_string().ends_with("Value") {
        let name_str = enum_name.to_string();
        let base_name = name_str.strip_suffix("Value").unwrap_or(&name_str);
        format_ident!("{}", base_name)
    } else {
        enum_name.clone()
    };

    let persistent_home_state_name = format_ident!("Persistent{}", home_state_name);

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = variant.fields {
            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            // Check for #[persistent] attribute
            let is_persistent = variant.attrs.iter().any(|attr| attr.path().is_ident("persistent"));

            // Generate ValueObject implementations
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

            // Collect persistent variants if this is HomeStateValue
            if is_persistent {
                persistent_variants.push(quote! {
                    #variant_name(#item_type, #value_type)
                });

                persistent_enum_variants.push(quote! {
                    #variant_name(#item_type)
                });

                persistent_conversion_matches.push(quote! {
                    #persistent_enum_name::#variant_name(item, value) => #enum_name::#variant_name(item, value)
                });

                persistent_state_conversion_matches.push(quote! {
                    #persistent_home_state_name::#variant_name(item) => #home_state_name::#variant_name(item)
                });
            }
        }
    }

    let persistent_generation = if !persistent_variants.is_empty() {
        quote! {
            #[derive(Debug, Clone, r#macro::EnumWithValue)]
            pub enum #persistent_enum_name {
                #(#persistent_variants),*
            }

            impl From<#persistent_enum_name> for #enum_name {
                fn from(val: #persistent_enum_name) -> Self {
                    match val {
                        #(#persistent_conversion_matches),*
                    }
                }
            }

            impl From<#persistent_home_state_name> for #home_state_name {
                fn from(val: #persistent_home_state_name) -> Self {
                    match val {
                        #(#persistent_state_conversion_matches),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        // Implement ValueObject for each variant
        #(#type_info_impls)*

        // Generate persistent types if applicable
        #persistent_generation
    };

    TokenStream::from(expanded)
}
