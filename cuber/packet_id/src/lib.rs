use proc_macro::TokenStream as RawToken;
use quote::quote;
use syn::{parse::Parse, parse_macro_input};

struct PacketIdArgs {
    id: syn::Expr,
}

impl Parse for PacketIdArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let id = syn::Expr::parse(input)?;
        
        Ok(Self { id })
    }
}

#[proc_macro_attribute]
pub fn sb_packet(args: RawToken, st: RawToken) -> RawToken {
    let args = parse_macro_input!(args as PacketIdArgs);
    let st = parse_macro_input!(st as syn::ItemStruct);

    let ident = st.ident.clone();
    let id = args.id;
    
    let token = quote! {
        #st
        impl ServerBoundPacket for #ident {
            const PACKET_ID: i32 = #id;
        }
    };

    token.into()
}
