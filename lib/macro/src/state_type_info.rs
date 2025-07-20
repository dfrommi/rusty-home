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

    // For HomeState implementations
    let mut home_state_to_f64_matches = Vec::new();
    let mut home_state_from_f64_matches = Vec::new();
    let mut home_state_data_point_matches = Vec::new();
    let mut home_state_data_frame_matches = Vec::new();

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

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            if *value { 1.0 } else { 0.0 }
                        }

                        fn from_f64(&self, value: f64) -> #value_type {
                            value > 0.0
                        }
                    }
                }
            } else {
                quote! {
                    impl crate::core::ValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            value.into()
                        }

                        fn from_f64(&self, value: f64) -> #value_type {
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

            // Generate HomeState ValueObject and DataPointAccess matches
            home_state_to_f64_matches.push(quote! {
                #enum_name::#variant_name(item, v) => item.to_f64(v)
            });

            home_state_from_f64_matches.push(quote! {
                #home_state_name::#variant_name(item) => {
                    #enum_name::#variant_name(item.clone(), item.from_f64(value))
                }
            });

            home_state_data_point_matches.push(quote! {
                #home_state_name::#variant_name(item) => {
                    let dp = item.current_data_point(api).await?;
                    Ok(dp.map_value(|v| #enum_name::#variant_name(item.clone(), v.clone())))
                }
            });

            home_state_data_frame_matches.push(quote! {
                #home_state_name::#variant_name(item) => item
                    .get_data_frame(range, api)
                    .await?
                    .map(|dp| #enum_name::#variant_name(item.clone(), dp.value.clone()))
            });
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

    // Generate HomeState implementations if this is HomeStateValue
    let home_state_implementations = if enum_name.to_string().ends_with("Value") {
        quote! {
            impl crate::core::ValueObject for #home_state_name {
                type ValueType = #enum_name;

                fn to_f64(&self, value: &Self::ValueType) -> f64 {
                    match value {
                        #(#home_state_to_f64_matches),*
                    }
                }

                fn from_f64(&self, value: f64) -> Self::ValueType {
                    match self {
                        #(#home_state_from_f64_matches),*
                    }
                }
            }

            impl crate::port::DataPointAccess<#home_state_name> for #home_state_name {
                async fn current_data_point(&self, api: &crate::core::HomeApi) -> anyhow::Result<crate::core::timeseries::DataPoint<#enum_name>> {
                    match self {
                        #(#home_state_data_point_matches),*
                    }
                }
            }

            impl crate::port::DataFrameAccess<#home_state_name> for #home_state_name {
                async fn get_data_frame(&self, range: crate::core::time::DateTimeRange, api: &crate::core::HomeApi) -> anyhow::Result<crate::core::timeseries::DataFrame<#enum_name>> {
                    let df: crate::core::timeseries::DataFrame<#enum_name> = match self {
                        #(#home_state_data_frame_matches),*
                    };

                    Ok(df)
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

        // Generate HomeState implementations if applicable
        #home_state_implementations
    };

    TokenStream::from(expanded)
}
