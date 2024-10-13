mod command_target_derive;
mod state_channel_derive;

use proc_macro::TokenStream;

#[proc_macro_derive(StateChannel)]
pub fn state_channel_derive(input: TokenStream) -> TokenStream {
    state_channel_derive::derive(input)
}

#[proc_macro_derive(CommandTarget)]
pub fn command_target_derive(input: TokenStream) -> TokenStream {
    command_target_derive::derive(input)
}
