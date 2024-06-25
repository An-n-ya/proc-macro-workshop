use darling::FromField;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataStruct, DeriveInput, Field, GenericArgument, Ident, Path,
    PathArguments, PathSegment, Type, TypePath,
};
#[derive(FromField, Clone)]
#[darling(attributes(builder))]
struct FieldAttributes {
    pub each: Option<String>,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident.clone();

    let builder_ident_str = format!("{}{}", struct_ident, "Builder");
    let builder_ident = Ident::new(&builder_ident_str, Span::call_site());

    let mut assign_fields = quote!();
    let mut builder_fields = quote!();
    let mut assign_methods = quote!();
    let mut build_statements = quote!();
    let mut build_fields = quote!();

    match &input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            for field in fields {
                let attribute = match FieldAttributes::from_field(&field) {
                    Ok(v) => v,
                    Err(e) => {
                        return TokenStream::from(e.write_errors());
                    }
                };

                let Field {
                    vis,
                    ident,
                    ty,
                    colon_token,
                    ..
                } = field;

                build_fields.extend(quote! {
                    #ident,
                });
                builder_fields.extend(quote! {
                    #vis #ident #colon_token std::option::Option<#ty>,
                });
                if let Some(argument_type) = arguments_of(ty, "Option") {
                    assign_methods.extend(quote! {
                        pub fn #ident(&mut self, value: #argument_type) -> &mut #builder_ident {
                            self.#ident = std::option::Option::Some(std::option::Option::Some(value));
                            self
                        }
                    });
                    assign_fields.extend(quote! {
                        #ident #colon_token std::option::Option::Some(std::option::Option::None),
                    });
                } else {
                    if let Some(each) = attribute.each {
                        let each_ident = Ident::new(&each, Span::call_site());
                        if let Some(argument_type) = arguments_of(ty, "Vec") {
                            assign_methods.extend(quote! {
                                pub fn #each_ident(&mut self, value: #argument_type) -> &mut #builder_ident {
                                    match self.#ident.as_mut() {
                                        std::option::Option::Some(vec) => vec.push(value),
                                        std::option::Option::None => self.#ident = std::option::Option::Some(vec![value]),
                                    }
                                    self
                                }
                            });
                            assign_fields.extend(quote! {
                                #ident #colon_token std::option::Option::Some(Vec::new()),
                            });
                        } else {
                            panic!("each attribute only works on Vec")
                        }
                    } else {
                        assign_methods.extend(quote! {
                            pub fn #ident(&mut self, value: #ty) -> &mut #builder_ident {
                                self.#ident = std::option::Option::Some(value);
                                self
                            }
                        });
                        assign_fields.extend(quote! {
                            #ident #colon_token std::option::Option::None,
                        });
                    }
                }
                build_statements.extend(quote! {
                    let #ident = if let std::option::Option::Some(val) = &self.#ident {
                        val.clone()
                    } else {
                        return std::option::Option::None;
                    };
                });
            }
        }
        _ => unimplemented!(),
    }

    quote! {
        pub struct #builder_ident {
            #builder_fields
        }
        impl #builder_ident {
            #assign_methods

            pub fn build(&self) -> std::option::Option<#struct_ident> {
                #build_statements
                std::option::Option::Some(#struct_ident {
                    #build_fields
                })
            }
        }
        impl #struct_ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #assign_fields
                }
            }
        }
    }
    .into()
}

#[allow(unused)]
fn arguments_of(typ: &Type, ident_s: &str) -> Option<GenericArgument> {
    match typ {
        Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) => match segments.first().unwrap() {
            PathSegment { ident, arguments } => match arguments {
                PathArguments::AngleBracketed(arguments) => {
                    if ident.to_string() == ident_s {
                        Some(arguments.args.first().unwrap().clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
        },
        _ => None,
    }
}
