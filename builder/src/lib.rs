use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let command_builder_type = Ident::new(&format!("{}Builder", name), Span::call_site());

    quote!(
        pub struct #command_builder_type {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }

        impl #name {
            pub fn builder() -> #command_builder_type {
                #command_builder_type {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }
        }
    )
    .into()
}

