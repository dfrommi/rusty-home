mod db_mapped;
mod enum_variants;
mod state_channel_derive;
mod typed_item;

use proc_macro::TokenStream;

#[proc_macro_derive(StateChannel)]
pub fn state_channel_derive(input: TokenStream) -> TokenStream {
    state_channel_derive::derive(input)
}

#[proc_macro_derive(DbMapped)]
pub fn db_mapped_derive(input: TokenStream) -> TokenStream {
    db_mapped::db_mapped(input)
}

#[proc_macro_derive(TypedItem)]
pub fn typed_item_derive(input: TokenStream) -> TokenStream {
    typed_item::derive_typed_item(input)
}

#[proc_macro_derive(TypedItemDelegation)]
pub fn typed_item_delegation_derive(input: TokenStream) -> TokenStream {
    typed_item::derive_typed_item_delegation(input)
}

#[proc_macro_derive(EnumVariants)]
pub fn enum_variants_derive(input: TokenStream) -> TokenStream {
    enum_variants::derive_typed_item(input)
}

fn enum_variants(data: syn::Data) -> Vec<syn::Variant> {
    // Ensure it's an enum
    if let syn::Data::Enum(data_enum) = data {
        data_enum.variants.into_iter().collect()
    } else {
        panic!("Macro can only be derived for enums");
    }
}
