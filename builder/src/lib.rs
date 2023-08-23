use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let command_builder_type = Ident::new(&format!("{}Builder", name), Span::call_site());

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = input.data
    {
        named
    } else {
        unimplemented!()
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #name: std::option::Option<#ty> }
    });

    let builder_fields_defaults = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name: Default::default() }
    });

    let setters = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        }
    });

    let set_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: self.#name.take().ok_or(
                format!("{0} not set; use method {0} to set the {0}'s value.", stringify!(#name))
            )?
        }
    });

    quote!(
        pub struct #command_builder_type {
            #(#builder_fields,)*
        }

        impl #command_builder_type {
            #(#setters)*

            pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
                Ok(#name {
                    #(#set_fields,)*
                })
            }
        }

        impl #name {
            pub fn builder() -> #command_builder_type {
                #command_builder_type {
                    #(#builder_fields_defaults,)*
                }
            }

        }
    )
    .into()
}
