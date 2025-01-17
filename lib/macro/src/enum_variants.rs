use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

pub fn derive_typed_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    // Ensure we're working with an enum
    let data_enum = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => panic!("Macro can only be derived for enums"),
    };

    let variants = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! { #enum_name::#variant_name }
    });

    let expanded = quote! {
        impl #enum_name {
            pub const fn variants() -> &'static [Self] {
                &[
                    #(#variants),*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}
