use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Ident};

pub fn derive_typed_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    // Ensure we're working with an enum
    let data_enum = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => panic!("TypedItem can only be derived for enums"),
    };

    // Generate type_name and item_name implementations
    let type_name = enum_name.to_string().to_snake_case();
    let variants = generate_variants_snake_case(&data_enum);

    let all_variants = generate_all_variants_array(&enum_name, &data_enum);
    let from_item_name_matches = generate_from_item_name_matches(&enum_name, &data_enum);

    let expanded = quote! {
        impl support::TypedItem for #enum_name {
            fn type_name(&self) -> &'static str {
                #type_name
            }

            fn item_name(&self) -> &'static str {
                match self {
                    #(#variants)*
                }
            }
        }

        impl #enum_name {
            pub const fn variants() -> &'static [Self] {
                &#all_variants
            }

            pub fn from_item_name(name: &str) -> Option<Self> {
                match name {
                    #(#from_item_name_matches)*
                    _ => None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn generate_variants_snake_case(data_enum: &DataEnum) -> Vec<proc_macro2::TokenStream> {
    data_enum
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let item_name = variant_name.to_string().to_snake_case();
            quote! {
                Self::#variant_name => #item_name,
            }
        })
        .collect()
}

fn generate_all_variants_array(
    enum_name: &Ident,
    data_enum: &DataEnum,
) -> proc_macro2::TokenStream {
    let variants = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! { #enum_name::#variant_name }
    });

    quote! {
        [
            #(#variants),*
        ]
    }
}

fn generate_from_item_name_matches(
    enum_name: &Ident,
    data_enum: &DataEnum,
) -> Vec<proc_macro2::TokenStream> {
    data_enum
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let item_name = variant_name.to_string().to_snake_case();
            quote! {
                #item_name => Some(#enum_name::#variant_name),
            }
        })
        .collect()
}
