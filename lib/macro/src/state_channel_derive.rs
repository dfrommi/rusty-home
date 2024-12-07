use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., ChannelValue
    let name = input.ident;

    // Ensure it's an enum
    let variants = if let Data::Enum(data_enum) = input.data {
        data_enum.variants
    } else {
        panic!("StateChannel macro can only be derived for enums");
    };

    // Collect the Channel enum variants and the implementations
    let mut channel_variants = Vec::new();
    let mut type_info_impls = Vec::new();
    let mut from_impls = Vec::new();
    let mut from_channel_value_impl = Vec::new();
    let mut into_dbvalue_impls = Vec::new();
    let mut typed_item_type_impls = Vec::new();
    let mut typed_item_item_impls = Vec::new();
    let mut from_type_and_item_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_snake_case();

        // Assume the variant has exactly two types in the tuple (like Temperature(DegreeCelsius, Temperature))
        if let syn::Fields::Unnamed(fields) = variant.fields {
            let type_1 = &fields.unnamed[0].ty;
            let type_2 = &fields.unnamed[1].ty;

            // Generate the variant for the Channel enum
            channel_variants.push(quote! {
                #variant_name(#type_1)
            });

            // Generate the ChannelTypeInfo implementation
            type_info_impls.push(quote! {
                impl ChannelTypeInfo for #type_1 {
                    type ValueType = #type_2;
                }
            });

            // Generate the From implementation
            from_impls.push(quote! {
                impl From<#type_1> for Channel {
                    fn from(val: #type_1) -> Self {
                        Channel::#variant_name(val)
                    }
                }

                impl From<&#type_1> for Channel {
                    fn from(val: &#type_1) -> Self {
                        Channel::#variant_name(val.clone())
                    }
                }
            });

            // Generate the From<&ChannelValue> for Channel implementation
            from_channel_value_impl.push(quote! {
                ChannelValue::#variant_name(id, _) => Channel::#variant_name(id.clone())
            });

            // Generate the Into<f64> implementation
            into_dbvalue_impls.push(quote! {
                ChannelValue::#variant_name(_, v) => v.into()
            });

            typed_item_type_impls.push(quote! {
                Channel::#variant_name(v) => v.type_name()
            });

            typed_item_item_impls.push(quote! {
                Channel::#variant_name(v) => v.item_name()
            });

            from_type_and_item_impls.push(quote! {
                #variant_name_snake => {
                    #variant_name::from_item_name(item_name).map(Channel::#variant_name)
                }
            });
        }
    }

    // The name of the new enum is Channel
    let channel_enum_name = format_ident!("Channel");

    let expanded = quote! {
        // Define the Channel enum
        #[derive(Debug, Clone, Hash, Eq, PartialEq)]
        pub enum #channel_enum_name {
            #(#channel_variants),*
        }

        // Implement ChannelTypeInfo for each variant
        #(#type_info_impls)*

        // Implement From for each variant
        #(#from_impls)*

        // Implement From<&ChannelValue> for Channel
        impl From<&#name> for #channel_enum_name {
            fn from(val: &#name) -> Self {
                match val {
                    #(#from_channel_value_impl),*
                }
            }
        }

        impl From<&#name> for crate::state::db::DbValue {
            fn from(val: &#name) -> Self {
                match val {
                    #(#into_dbvalue_impls),*
                }
            }
        }

        impl support::TypedItem for #channel_enum_name {
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


        impl #channel_enum_name {
            pub fn from_type_and_item(type_name: &str, item_name: &str) -> Option<Self> {
                match type_name {
                    #(#from_type_and_item_impls),*
                    _ => None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
