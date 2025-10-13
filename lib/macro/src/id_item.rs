use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Ident, Type, parse_macro_input};

pub fn derive_id_item(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let type_name = input.ident;
    let type_name_ext = type_name.to_string().to_snake_case();

    let expanded = match input.data {
        Data::Enum(data_enum) => derive_enum(&type_name, &type_name_ext, data_enum),
        Data::Struct(data_struct) => derive_struct(&type_name, &type_name_ext, data_struct),
        Data::Union(_) => panic!("Id derive does not support unions"),
    };

    TokenStream::from(expanded)
}

fn derive_enum(enum_name: &Ident, type_name_ext: &str, data_enum: DataEnum) -> proc_macro2::TokenStream {
    let mut ext_id_statics = Vec::new();
    let mut ext_id_matches = Vec::new();
    let mut from_ext_item_name_matches = Vec::new();

    for variant in data_enum.variants {
        let variant_name = &variant.ident;
        let variant_name_ext = variant_name.to_string().to_snake_case();

        match variant.fields {
            Fields::Unit => {
                let static_suffix = variant_name.to_string().to_shouty_snake_case();
                let ext_id_static_name = Ident::new(&format!("{}_EXT_ID", static_suffix), variant_name.span());

                ext_id_statics.push(quote! {
                    static #ext_id_static_name: crate::core::id::ExternalId = crate::core::id::ExternalId::new_static(#type_name_ext, #variant_name_ext);
                });
                ext_id_matches.push(quote! {
                    #enum_name::#variant_name => #ext_id_static_name.clone()
                });

                from_ext_item_name_matches.push(quote! {
                    #variant_name_ext => #enum_name::#variant_name
                });
            }
            Fields::Unnamed(fields) => {
                let bindings: Vec<_> = (0..fields.unnamed.len())
                    .map(|idx| format_ident!("field_{idx}"))
                    .collect();

                let variant_segments = bindings.iter().map(|binding| {
                    quote! { #binding.ext_id().variant_name() }
                });

                ext_id_matches.push(quote! {
                    #enum_name::#variant_name(#(#bindings),*) => {
                        let mut segments = vec![#variant_name_ext.to_string()];
                        #(segments.push(#variant_segments.to_string());)*
                        let variant_name = segments.join("::");
                        crate::core::id::ExternalId::new(#type_name_ext, variant_name)
                    }
                });

                if fields.unnamed.len() == 1 {
                    let field_type = &fields.unnamed[0].ty;

                    if let Type::Path(type_path) = field_type {
                        if let Some(segment) = type_path.path.segments.last() {
                            if segment.arguments.is_empty() {
                                let nested_type_ext = segment.ident.to_string().to_snake_case();

                                from_ext_item_name_matches.push(quote! {
                                    variant_name if variant_name.starts_with(concat!(#variant_name_ext, "::")) => {
                                        let nested_variant = match variant_name.split_once("::") {
                                            Some((_, nested)) if !nested.is_empty() => nested,
                                            _ => anyhow::bail!("Error converting ExternalId, missing nested variant name for {}", variant_name),
                                        };
                                        let nested_id = crate::core::id::ExternalId::new(#nested_type_ext, nested_variant);
                                        let nested_item = #field_type::try_from(nested_id)?;
                                        #enum_name::#variant_name(nested_item)
                                    }
                                });
                            }
                        }
                    }
                }
            }
            Fields::Named(fields) => {
                let bindings: Vec<_> = fields
                    .named
                    .into_iter()
                    .map(|field| field.ident.expect("named field expected"))
                    .collect();

                let variant_segments = bindings.iter().map(|binding| {
                    quote! { #binding.ext_id().variant_name() }
                });

                ext_id_matches.push(quote! {
                    #enum_name::#variant_name { #(#bindings),* } => {
                        let mut segments = vec![#variant_name_ext.to_string()];
                        #(segments.push(#variant_segments.to_string());)*
                        let variant_name = segments.join("::");
                        crate::core::id::ExternalId::new(#type_name_ext, variant_name)
                    }
                });
            }
        }
    }

    let try_from_impl = if from_ext_item_name_matches.is_empty() {
        quote! {}
    } else {
        quote! {
            impl TryFrom<&crate::core::id::ExternalId> for #enum_name {
                type Error = anyhow::Error;

                fn try_from(value: &crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                    if value.type_name() != #type_name_ext {
                        anyhow::bail!("Error converting ExternalId, expected type {}, got {}", #type_name_ext, value.type_name());
                    }

                    let item = match value.variant_name() {
                        #(#from_ext_item_name_matches),*,
                        _ => anyhow::bail!("Error converting ExternalId, unknown name {}", value.variant_name()),
                    };

                    Ok(item)
                }
            }

            impl TryFrom<crate::core::id::ExternalId> for #enum_name {
                type Error = anyhow::Error;

                fn try_from(value: crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                    Self::try_from(&value)
                }
            }
        }
    };

    quote! {
        impl #enum_name {
            pub fn ext_id(&self) -> crate::core::id::ExternalId {
                #(#ext_id_statics)*

                match self {
                    #(#ext_id_matches),*
                }
            }
        }

        #try_from_impl
    }
}

fn derive_struct(struct_name: &Ident, type_name_ext: &str, data_struct: DataStruct) -> proc_macro2::TokenStream {
    match data_struct.fields {
        Fields::Unit => {
            Error::new_spanned(struct_name, "Id derive requires structs to have at least one field").to_compile_error()
        }
        Fields::Unnamed(fields) => {
            let indices: Vec<_> = (0..fields.unnamed.len()).collect();
            let variant_parts = indices.iter().map(|idx| {
                let index = syn::Index::from(*idx);
                quote! { self.#index.ext_id().variant_name() }
            });

            quote! {
                impl #struct_name {
                    pub fn ext_id(&self) -> crate::core::id::ExternalId {
                        let variant_name = vec![#(#variant_parts),*].join("::");
                        crate::core::id::ExternalId::new(#type_name_ext, variant_name)
                    }
                }
            }
        }
        Fields::Named(fields) => {
            let field_idents: Vec<_> = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field expected").clone())
                .collect();

            let variant_parts = field_idents.iter().map(|field_ident| {
                quote! { self.#field_ident.ext_id().variant_name() }
            });

            quote! {
                impl #struct_name {
                    pub fn ext_id(&self) -> crate::core::id::ExternalId {
                        let variant_name = vec![#(#variant_parts),*].join("::");
                        crate::core::id::ExternalId::new(#type_name_ext, variant_name)
                    }
                }
            }
        }
    }
}

pub fn derive_id_item_delegation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the input enum, e.g., ChannelValue
    let name = input.ident;

    // Ensure it's an enum
    let variants = super::enum_variants(input.data);

    let mut ext_id_matches = Vec::new();
    let mut try_from_impls = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        ext_id_matches.push(quote! {
            #name::#variant_name(v) => v.ext_id()
        });

        //should always be true
        if let syn::Fields::Unnamed(fields) = variant.fields {
            let item_type = &fields.unnamed[0].ty;

            try_from_impls.push(quote! {
                if let Ok(item) = #item_type::try_from(value) {
                    return Ok(#name::#variant_name(item));
                }
            });
        }
    }

    let expanded = quote! {
        impl #name {
            pub fn ext_id(&self) -> crate::core::id::ExternalId {
                match self {
                    #(#ext_id_matches),*
                }
            }
        }

        impl TryFrom<&crate::core::id::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: &crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                #(#try_from_impls)*
                anyhow::bail!("Error converting ExternalId, unknown type/name {}/{}", value.type_name(), value.variant_name());
            }
        }

        impl TryFrom<crate::core::id::ExternalId> for #name {
            type Error = anyhow::Error;

            fn try_from(value: crate::core::id::ExternalId) -> Result<Self, Self::Error> {
                Self::try_from(&value)
            }
        }
    };

    TokenStream::from(expanded)
}
