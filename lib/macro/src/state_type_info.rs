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

    TokenStream::from(quote! {
        #annotated_generation
    })
}

fn generate_annotated_enum(
    enum_name: &syn::Ident,
    home_state_name: &syn::Ident,
    variants: &[syn::Variant],
) -> TokenStream2 {
    let mut item_value_object_impls = Vec::new();
    let mut state_value_matches = Vec::new();
    let mut home_state_project_matches = Vec::new();

    for variant in variants {
        if let syn::Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() < 2 {
                continue;
            }

            let variant_name = &variant.ident;
            let item_type = &fields.unnamed[0].ty;
            let value_type = &fields.unnamed[1].ty;

            let state_value_variant = if is_bool_type(value_type) {
                format_ident!("Boolean")
            } else {
                value_type_of_variant(value_type)
            };

            if is_bool_type(value_type) {
                state_value_matches.push(quote! {
                    #enum_name::#variant_name(_, value) => {
                        crate::home::state::StateValue::Boolean(*value)
                    }
                });
            } else {
                state_value_matches.push(quote! {
                    #enum_name::#variant_name(_, value) => {
                        crate::home::state::StateValue::#state_value_variant(value.clone())
                    }
                });
            }

            item_value_object_impls.push(quote! {
                impl crate::port::ValueObject for #item_type {
                    type ValueType = #value_type;

                    fn project_state_value(&self, value: crate::home::state::StateValue) -> Option<#value_type> {
                        match value {
                            crate::home::state::StateValue::#state_value_variant(v) => Some(v),
                            _ => None,
                        }
                    }

                    fn as_state_value(value: #value_type) -> crate::home::state::StateValue {
                        crate::home::state::StateValue::#state_value_variant(value)
                    }
                }
            });

            home_state_project_matches.push(quote! {
                (#home_state_name::#variant_name(item), crate::home::state::StateValue::#state_value_variant(value)) => {
                    Some(#enum_name::#variant_name(item.clone(), value))
                }
            });
        }
    }

    let home_state_impls = quote! {
        impl crate::port::ValueObject for #home_state_name {
            type ValueType = #enum_name;

            fn project_state_value(&self, value: crate::home::state::StateValue) -> Option<Self::ValueType> {
                match (self, value) {
                    #(#home_state_project_matches),*,
                    _ => None,
                }
            }

            fn as_state_value(value: Self::ValueType) -> crate::home::state::StateValue {
                value.value()
            }
        }

        impl #enum_name {
            pub fn value(&self) -> crate::home::state::StateValue {
                match self {
                    #(#state_value_matches),*
                }
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
