use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    self, parse_macro_input, AngleBracketedGenericArguments, DeriveInput, Expr, GenericArgument,
    Lit, MetaNameValue, Path, PathArguments, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
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

    let ty_is_option = |ty: &Type| {
        if let Type::Path(
            TypePath {
                path: Path { ref segments, .. },
                ..
            },
            ..,
        ) = ty
        {
            // the path could be std::option::Option also so taking just the last segment
            return segments.len() > 0 && segments.last().unwrap().ident.to_string() == "Option";
        }
        false
    };

    let get_angle_bracket_type_arg = |ty: &Type| -> Option<Type> {
        if let Type::Path(
            TypePath {
                path: Path { ref segments, .. },
                ..
            },
            ..,
        ) = ty
        {
            // the path could be std::option::Option also so taking just the last segment
            // and its inside angle bracketed args
            let last_segment = segments.last().unwrap();
            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                ref args, ..
            }) = last_segment.arguments
            {
                if let GenericArgument::Type(ref inner_type) = args[0] {
                    return Some(inner_type.clone());
                }
            }
        }
        None
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        // assume that fields with this attribute have the type Vec stated in the test 07
        if f.attrs.len() > 0 {
            let vec_ty = get_angle_bracket_type_arg(ty);
            return quote! { #name: Vec<#vec_ty> };
        }

        if ty_is_option(ty) {
            return quote! { #name: #ty };
        }

        quote! { #name: std::option::Option<#ty> }
    });

    let builder_fields_defaults = fields.iter().map(|f| {
        let name = &f.ident;
        quote! { #name: Default::default() }
    });

    let custom_attribute_methods = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let attrs = &f.attrs;
        if !attrs.is_empty() {
            for attr in attrs.iter() {
                // #[builder(each = "arg")]
                //   ^^^^^^^ <- (ident)
                if attr.path().is_ident("builder") {
                    let MetaNameValue { path, value, .. }: syn::MetaNameValue =
                        attr.parse_args().unwrap();
                    // #[builder(each = "arg")]
                    //           ^^^^ <- (it's a path, can also have double colon separated path like xyz::each)
                    if path.segments.len() > 0 && path.segments.last().unwrap().ident == "each" {
                        // #[builder(each = "arg")]
                        //           ^^^^^^^^^^^^ <- (expr)
                        if let Expr::Lit(expr) = value {
                            // #[builder(each = "arg")]
                            //                  ^^^^^ <- (literal)
                            if let Lit::Str(literal) = expr.lit {
                                let ident = Ident::new(literal.value().as_str(), Span::call_site());
                                let type_inside_vec = get_angle_bracket_type_arg(ty).unwrap();
                                return Some(quote! {
                                    pub fn #ident(&mut self, #ident: #type_inside_vec) -> &mut Self {
                                        self.#name.push(#ident);
                                        self
                                    }
                                });
                            }
                        }
                    }
                }
            }
            unimplemented!()
        }
        None
    });

    let setters = fields.iter().filter_map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        let attrs = &f.attrs;
        if attrs.is_empty() {
            Some(if ty_is_option(&ty) {
                // extract the type inside Option<type>
                let opt_inner_ty = get_angle_bracket_type_arg(&ty).unwrap();
                quote! {
                    pub fn #name(&mut self, #name: #opt_inner_ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                }
            } else {
                quote! {
                    pub fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                }
            })
        } else {
            None
        }
    });

    let set_fields = fields.iter().filter_map(|f| {
        let name = &f.ident;

        // fields with custom attrs are vec
        if f.attrs.len() > 0 {
            return Some(quote! { #name: self.#name.clone() });
        }

        if ty_is_option(&f.ty) {
            return Some(quote! {
                #name: self.#name.take()
            });
        }

        Some(quote! {
            #name: self.#name.take().ok_or(
                format!("{0} not set; use method {0} to set the {0}'s value.", stringify!(#name))
            )?
        })
    });

    quote!(
        pub struct #command_builder_type {
            #(#builder_fields,)*
        }

        impl #command_builder_type {
            #(#setters)*
            #(#custom_attribute_methods)*
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
