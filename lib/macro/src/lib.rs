mod enum_variants;
mod id_item;
mod state_enum;
mod state_trace;

use proc_macro::TokenStream;

#[proc_macro_derive(StateEnumDerive)]
pub fn state_enum_derive(input: TokenStream) -> TokenStream {
    state_enum::derive(input)
}

#[proc_macro_derive(Id)]
pub fn id_item_derive(input: TokenStream) -> TokenStream {
    id_item::derive_id_item(input)
}

#[proc_macro_derive(IdDelegation)]
pub fn id_item_delegation_derive(input: TokenStream) -> TokenStream {
    id_item::derive_id_item_delegation(input)
}

#[proc_macro_derive(EnumVariants)]
pub fn enum_variants_derive(input: TokenStream) -> TokenStream {
    enum_variants::derive_typed_item(input)
}

#[proc_macro_attribute]
pub fn trace_state(attr: TokenStream, item: TokenStream) -> TokenStream {
    state_trace::trace_state_access(attr, item)
}

fn enum_variants(data: syn::Data) -> Vec<syn::Variant> {
    // Ensure it's an enum
    if let syn::Data::Enum(data_enum) = data {
        data_enum.variants.into_iter().collect()
    } else {
        panic!("Macro can only be derived for enums");
    }
}
