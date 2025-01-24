use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Passable)]
pub fn derive_passable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let result = quote! {
        impl dsbuild_message::Typped for #name {
            const TYPE: &str = stringify!(#name);
        }

        impl From<dsbuild_message::Message> for #name {
            fn from(message: dsbuild_message::Message) -> Self {
                message.get_data().unwrap()
            }
        }
    };

    result.into()
}
