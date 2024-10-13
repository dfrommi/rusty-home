use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., Command
    let name = input.ident;

    // Ensure it's an enum
    let variants = if let Data::Enum(data_enum) = input.data {
        data_enum.variants
    } else {
        panic!("CommandTargetMacro can only be derived for enums");
    };

    // Collect the CommandTarget enum variants
    let mut target_variants = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        // Process the fields of each variant
        if let Fields::Named(fields) = variant.fields {
            // Look for the `device` field and extract its type
            for field in fields.named {
                if field.ident.as_ref().unwrap() == "device" {
                    let device_type = &field.ty;

                    // Generate the variant for the CommandTarget enum
                    target_variants.push(quote! {
                        #variant_name(#device_type)
                    });
                }
            }
        }
    }

    // The name of the new enum is CommandTarget
    let target_enum_name = format_ident!("CommandTarget");

    let expanded = quote! {
        // Define the CommandTarget enum
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "type", content = "device", rename_all = "snake_case")]
        pub enum #target_enum_name {
            #(#target_variants),*
        }
    };

    TokenStream::from(expanded)
}
