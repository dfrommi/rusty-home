mod state_channel_derive;
mod typed_item;

use proc_macro::TokenStream;

#[proc_macro_derive(StateChannel)]
pub fn state_channel_derive(input: TokenStream) -> TokenStream {
    state_channel_derive::derive(input)
}

#[proc_macro_derive(TypedItem)]
pub fn typed_item_derive(input: TokenStream) -> TokenStream {
    typed_item::derive_typed_item(input)
}
