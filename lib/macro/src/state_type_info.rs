use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;
    let variants = super::enum_variants(input.data);

    let persistent_enum_name = format_ident!("Persistent{}", enum_name);

    let home_state_name = if enum_name.to_string().ends_with("Value") {
        let name_str = enum_name.to_string();
        let base_name = name_str.strip_suffix("Value").unwrap_or(&name_str);
        format_ident!("{}", base_name)
    } else {
        enum_name.clone()
    };

    let persistent_home_state_name = format_ident!("Persistent{}", home_state_name);

    let annotated_generation = generate_annotated_enum(enum_name, &home_state_name, &variants);
    let persistent_generation = generate_persistent_enum(&persistent_enum_name, &persistent_home_state_name, &variants);

    TokenStream::from(quote! {
        #annotated_generation
        #persistent_generation
    })
}

fn generate_annotated_enum(
    enum_name: &syn::Ident,
    home_state_name: &syn::Ident,
    variants: &[syn::Variant],
) -> TokenStream2 {
    let mut item_value_object_impls = Vec::new();
    let mut enum_value_to_f64_matches = Vec::new();
    let mut home_state_data_point_matches = Vec::new();
    let mut home_state_data_frame_matches = Vec::new();

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() < 2 {
                continue;
            }

            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            if is_bool_type(value_type) {
                item_value_object_impls.push(quote! {
                    impl crate::core::ValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            if *value { 1.0 } else { 0.0 }
                        }
                    }
                });
            } else {
                item_value_object_impls.push(quote! {
                    impl crate::core::ValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            value.into()
                        }
                    }
                });
            }

            enum_value_to_f64_matches.push(quote! {
                #enum_name::#variant_name(item, v) => {
                    <#item_type as crate::core::ValueObject>::to_f64(&item, &v)
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

    let home_state_impls = if enum_value_to_f64_matches.is_empty() {
        quote! {}
    } else {
        quote! {
            impl crate::core::ValueObject for #home_state_name {
                type ValueType = #enum_name;

                fn to_f64(&self, value: &Self::ValueType) -> f64 {
                    match value {
                        #(#enum_value_to_f64_matches),*
                    }
                }
            }

            impl #enum_name {
                pub fn value_to_f64(&self) -> f64 {
                    match self {
                        #(#enum_value_to_f64_matches),*
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
    };

    quote! {
        #(#item_value_object_impls)*
        #home_state_impls
    }
}

fn generate_persistent_enum(
    persistent_enum_name: &syn::Ident,
    persistent_home_state_name: &syn::Ident,
    variants: &[syn::Variant],
) -> TokenStream2 {
    let mut item_persistent_impls = Vec::new();
    let mut persistent_variants = Vec::new();
    let mut persistent_state_to_f64_matches = Vec::new();
    let mut persistent_state_from_f64_matches = Vec::new();

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() < 2 {
                continue;
            }

            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;
            let is_persistent = variant.attrs.iter().any(|attr| attr.path().is_ident("persistent"));

            if is_bool_type(value_type) {
                item_persistent_impls.push(quote! {
                    impl crate::core::PersistentValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            if *value { 1.0 } else { 0.0 }
                        }

                        fn from_f64(&self, value: f64) -> #value_type {
                            value > 0.0
                        }
                    }
                });
            } else {
                item_persistent_impls.push(quote! {
                    impl crate::core::PersistentValueObject for #item_type {
                        type ValueType = #value_type;

                        fn to_f64(&self, value: &#value_type) -> f64 {
                            value.into()
                        }

                        fn from_f64(&self, value: f64) -> #value_type {
                            value.into()
                        }
                    }
                });
            }

            if is_persistent {
                persistent_variants.push(quote! {
                    #variant_name(#item_type, #value_type)
                });

                persistent_state_to_f64_matches.push(quote! {
                    #persistent_enum_name::#variant_name(item, value) => {
                        <#item_type as crate::core::PersistentValueObject>::to_f64(&item, &value)
                    }
                });

                persistent_state_from_f64_matches.push(quote! {
                    #persistent_home_state_name::#variant_name(item) => {
                        #persistent_enum_name::#variant_name(
                            item.clone(),
                            <#item_type as crate::core::PersistentValueObject>::from_f64(&item, value)
                        )
                    }
                });
            }
        }
    }

    let persistent_enum_impls = if persistent_variants.is_empty() {
        quote! {}
    } else {
        quote! {
            #[derive(Debug, Clone, r#macro::EnumWithValue)]
            pub enum #persistent_enum_name {
                #(#persistent_variants),*
            }

            impl #persistent_enum_name {
                pub fn value_to_f64(&self) -> f64 {
                    match self {
                        #(#persistent_state_to_f64_matches),*
                    }
                }
            }

            impl #persistent_home_state_name {
                pub fn with_value_f64(&self, value: f64) -> #persistent_enum_name {
                    match self {
                        #(#persistent_state_from_f64_matches),*
                    }
                }
            }

            impl crate::core::PersistentValueObject for #persistent_home_state_name {
                type ValueType = #persistent_enum_name;

                fn to_f64(&self, value: &Self::ValueType) -> f64 {
                    match value {
                        #(#persistent_state_to_f64_matches),*
                    }
                }

                fn from_f64(&self, value: f64) -> Self::ValueType {
                    match self {
                        #(#persistent_state_from_f64_matches),*
                    }
                }
            }
        }
    };

    quote! {
        #(#item_persistent_impls)*
        #persistent_enum_impls
    }
}

fn is_bool_type(value_type: &syn::Type) -> bool {
    match value_type {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map_or(false, |segment| segment.ident == "bool"),
        _ => false,
    }
}
