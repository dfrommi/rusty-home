use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn derive_typed_item(input: TokenStream) -> TokenStream {
    // Parse the input tokens
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    // Ensure we're working with an enum
    let data_enum = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => panic!("Macro can only be derived for enums"),
    };

    let mut unit_variants = Vec::new();
    let mut nested_variants = Vec::new();

    for variant in data_enum.variants.iter() {
        let variant_name = &variant.ident;
        
        match &variant.fields {
            // Variant with no fields (current behavior)
            Fields::Unit => {
                unit_variants.push(quote! { #enum_name::#variant_name });
            }
            // Variant with exactly one unnamed field
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().unwrap().ty;
                
                // Generate combinations by calling variants() on the nested enum
                nested_variants.push(quote! {
                    #field_type::variants().iter().map(|inner| #enum_name::#variant_name(inner.clone()))
                });
            }
            // Variant with named fields or multiple unnamed fields - not supported
            _ => {
                panic!("EnumVariants macro only supports unit variants or variants with exactly one unnamed field");
            }
        }
    }

    // Check if we have any nested variants (variants with fields)
    let has_nested = data_enum.variants.iter().any(|variant| {
        matches!(variant.fields, Fields::Unnamed(ref fields) if fields.unnamed.len() == 1)
    });

    let expanded = if has_nested {
        // If we have nested variants, we need to collect all combinations
        quote! {
            impl #enum_name {
                pub fn variants() -> Vec<Self> {
                    let mut result = Vec::new();
                    
                    // Add unit variants
                    #(
                        result.push(#unit_variants);
                    )*
                    
                    // Add nested variants
                    #(
                        result.extend(#nested_variants);
                    )*
                    
                    result
                }
            }
        }
    } else {
        // If all variants are unit variants, use the original const implementation
        quote! {
            impl #enum_name {
                pub const fn variants() -> &'static [Self] {
                    &[
                        #(#unit_variants),*
                    ]
                }
            }
        }
    };

    TokenStream::from(expanded)
}
