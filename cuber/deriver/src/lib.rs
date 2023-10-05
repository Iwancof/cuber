extern crate proc_macro;

use proc_macro::TokenStream as RawToken;
use proc_macro2::TokenStream;
use syn::{parse_macro_input, Fields};

use quote::quote;

#[proc_macro_derive(Encodable)]
pub fn derive_encodable(input: RawToken) -> RawToken {
    let st = parse_macro_input!(input as syn::DeriveInput);

    let ident = st.ident.clone();

    let write_code = match st.data {
        syn::Data::Struct(ref str) => match str.fields {
            Fields::Unit => quote! {},
            Fields::Named(ref names) => {
                let encode_part = names
                    .named
                    .iter()
                    .map(|name| {
                        let name = name
                            .ident
                            .clone()
                            .expect("(TODO) unnamed structure is not allowed");
                        quote! {
                            __written += self.#name.encode(writer);
                        }
                    })
                    .fold(quote! {}, |acc, mem| quote! { #acc #mem });

                quote! {
                    let mut __written = 0;
                    #encode_part
                    __written
                }
            }
            Fields::Unnamed(ref _unnamed) => {
                todo!()
            }
        },
        _ => {
            panic!("encodable derive only support struct");
        }
    };

    quote! {
        impl Encodable for #ident {
            fn encode<T: std::io::Write>(&self, writer: &mut T) -> usize {
                #write_code
            }
        }
    }
    .into()
}

#[proc_macro_derive(Decodable)]
pub fn derive_decoable(input: RawToken) -> RawToken {
    let st = parse_macro_input!(input as syn::DeriveInput);

    let ident = st.ident.clone();

    let read_code = match st.data {
        syn::Data::Struct(ref str) => match str.fields {
            Fields::Unit => quote! {},
            Fields::Named(ref names) => {
                let decode_part = names
                    .named
                    .iter()
                    .map(|name| {
                        let name = name
                            .ident
                            .clone()
                            .expect("(TODO) unnamed structure is not allowed");
                        quote! {
                            let #name = Decodable::decode(reader).with_context(|| format!("Failed to decode {}", stringify!(#name)))?;
                        }
                    })
                    .fold(TokenStream::new(), |acc, mem| quote! { #acc #mem });

                let construct_part = names
                    .named
                    .iter()
                    .map(|name| {
                        let name = name
                            .ident
                            .clone()
                            .expect("(TODO) unnamed structure is not allowed");
                        quote! {
                            #name,
                        }
                    })
                    .fold(quote! {}, |acc, mem| quote! { #acc #mem });

                quote! {
                    #decode_part
                    Ok(Self {
                        #construct_part
                    })
                }
            }
            Fields::Unnamed(ref _unnamed) => {
                todo!()
            }
        },
        _ => {
            panic!("encodable derive only support struct");
        }
    };

    // dbg!(read_code.to_string());

    quote! {
        impl Decodable for #ident {
            fn decode<T: std::io::Read>(reader: &mut T) -> anyhow::Result<Self> {
                #read_code
            }
        }
    }
    .into()
}
