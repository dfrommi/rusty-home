use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;
    let variants = super::enum_variants(input.data);

    let home_state_name = format_ident!(
        "{}",
        enum_name
            .to_string()
            .strip_suffix("Value")
            .expect("Expected input enum to end with Value")
    );
    let annotated_generation = generate_annotated_enum(enum_name, &home_state_name, &variants);

    let persistent_enum_name = format_ident!("Persistent{}", enum_name);
    let persistent_home_state_name = format_ident!("Persistent{}", home_state_name);
    let persistent_variants: Vec<syn::Variant> = variants
        .clone()
        .into_iter()
        .filter(|variant| variant.attrs.iter().any(|attr| attr.path().is_ident("persistent")))
        .collect();
    let persistent_generation =
        generate_persistent_enum(&persistent_enum_name, &persistent_home_state_name, &persistent_variants);

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
    let mut home_state_data_point_matches = Vec::new();
    let mut home_state_data_frame_matches = Vec::new();
    let mut state_value_matches = Vec::new();

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() < 2 {
                continue;
            }

            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            if is_bool_type(value_type) {
                state_value_matches.push(quote! {
                    #enum_name::#variant_name(_, value) => {
                        crate::home::state::StateValue::Boolean(*value)
                    }
                });
            } else {
                let state_value_variant = value_type_of_variant(value_type);
                state_value_matches.push(quote! {
                    #enum_name::#variant_name(_, value) => {
                        crate::home::state::StateValue::#state_value_variant(value.clone())
                    }
                });
            }

            item_value_object_impls.push(quote! {
                impl crate::port::ValueObject for #item_type {
                    type ValueType = #value_type;
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

    let home_state_impls = quote! {
        impl crate::port::ValueObject for #home_state_name {
            type ValueType = #enum_name;
        }

        impl #enum_name {
            pub fn value(&self) -> crate::home::state::StateValue {
                match self {
                    #(#state_value_matches),*
                }
            }
        }

        impl crate::port::DataPointAccess<#enum_name> for #home_state_name {
            async fn current_data_point(&self, api: &crate::core::HomeApi) -> anyhow::Result<crate::core::timeseries::DataPoint<#enum_name>> {
                match self {
                    #(#home_state_data_point_matches),*
                }
            }
        }

        impl crate::port::DataFrameAccess<#enum_name> for #home_state_name {
            async fn get_data_frame(&self, range: crate::core::time::DateTimeRange, api: &crate::core::HomeApi) -> anyhow::Result<crate::core::timeseries::DataFrame<#enum_name>> {
                let df: crate::core::timeseries::DataFrame<#enum_name> = match self {
                    #(#home_state_data_frame_matches),*
                };

                Ok(df)
            }
        }
    };

    let no_value_enum = generate_enum_without_value(enum_name, variants);

    quote! {
        #no_value_enum
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
    let mut persistent_state_value_matches = Vec::new();

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() < 2 {
                continue;
            }

            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            if is_bool_type(value_type) {
                persistent_state_value_matches.push(quote! {
                    #persistent_enum_name::#variant_name(_, value) => {
                        crate::home::state::PersistentStateValue::Boolean(*value)
                    }
                });

                item_persistent_impls.push(quote! {
                    impl crate::home::state::PersistentHomeStateTypeInfo for #item_type {
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
                let state_value_variant = value_type_of_variant(value_type);
                persistent_state_value_matches.push(quote! {
                    #persistent_enum_name::#variant_name(_, value) => {
                        crate::home::state::PersistentStateValue::#state_value_variant(value.clone())
                    }
                });

                item_persistent_impls.push(quote! {
                    impl crate::home::state::PersistentHomeStateTypeInfo for #item_type {
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

            persistent_variants.push(quote! {
                #variant_name(#item_type, #value_type)
            });

            persistent_state_to_f64_matches.push(quote! {
                #persistent_enum_name::#variant_name(item, value) => {
                    <#item_type as crate::home::state::PersistentHomeStateTypeInfo>::to_f64(&item, &value)
                }
            });

            persistent_state_from_f64_matches.push(quote! {
                #persistent_home_state_name::#variant_name(item) => {
                    #persistent_enum_name::#variant_name(
                        item.clone(),
                        <#item_type as crate::home::state::PersistentHomeStateTypeInfo>::from_f64(&item, value)
                    )
                }
            });
        }
    }

    let persistent_enum_impls = quote! {
        #[derive(Debug, Clone)]
        pub enum #persistent_enum_name {
            #(#persistent_variants),*
        }

        impl #persistent_enum_name {
            pub fn value(&self) -> crate::home::state::PersistentStateValue {
                match self {
                    #(#persistent_state_value_matches),*
                }
            }

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

        impl crate::home::state::PersistentHomeStateTypeInfo for #persistent_home_state_name {
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
    };

    let no_value_enum = generate_enum_without_value(persistent_enum_name, variants);

    quote! {
        #no_value_enum
        #(#item_persistent_impls)*
        #persistent_enum_impls
    }
}

fn generate_enum_without_value(value_enum_name: &syn::Ident, variants: &[syn::Variant]) -> TokenStream2 {
    let target_enum_name = format_ident!(
        "{}",
        value_enum_name
            .to_string()
            .strip_suffix("Value")
            .expect("Expected input enum to end with Value")
    );

    let mut target_variants = Vec::new();
    let mut item_to_target_impls = Vec::new();
    let mut source_to_target_impl = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        if let syn::Fields::Unnamed(fields) = &variant.fields {
            let item_type = &fields.unnamed[0].ty;
            let _value_type = &fields.unnamed[1].ty;

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
            });

            // Generate the From<&ChannelValue> for Channel implementation
            source_to_target_impl.push(quote! {
                #value_enum_name::#variant_name(id, _) => #target_enum_name::#variant_name(id.clone())
            });
        }
    }

    quote! {
        // Define the Channel enum
        #[derive(Debug, Clone, Hash, Eq, PartialEq, r#macro::EnumVariants, r#macro::IdDelegation)]
        pub enum #target_enum_name {
            #(#target_variants),*
        }

        // Implement From for each variant
        #(#item_to_target_impls)*

        // Implement From<&ChannelValue> for Channel
        impl From<&#value_enum_name> for #target_enum_name {
            fn from(val: &#value_enum_name) -> Self {
                match val {
                    #(#source_to_target_impl),*
                }
            }
        }
    }
}

fn is_bool_type(value_type: &syn::Type) -> bool {
    value_type_of_variant(value_type) == "bool"
}

fn value_type_of_variant(value_type: &syn::Type) -> syn::Ident {
    match value_type {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.clone())
            .expect("StateTypeInfoDerive: expected at least one path segment for value type"),
        _ => panic!("StateTypeInfoDerive: unsupported value type for value() generation"),
    }
}
