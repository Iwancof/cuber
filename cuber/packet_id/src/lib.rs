use proc_macro::TokenStream as RawToken;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct SBPacketIdArgs {
    id: syn::Expr,
}

impl Parse for SBPacketIdArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let id = syn::Expr::parse(input)?;

        Ok(Self { id })
    }
}

#[proc_macro_attribute]
pub fn sb_packet(args: RawToken, st: RawToken) -> RawToken {
    let args = parse_macro_input!(args as SBPacketIdArgs);
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

struct CBPacketArgs {
    state: syn::Expr,
    id: syn::Expr,
}

impl Parse for CBPacketArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let state = syn::Expr::parse(input)?;
        let _comma = syn::token::Comma::parse(input)?;
        let id = syn::Expr::parse(input)?;

        Ok(Self { state, id })
    }
}

#[proc_macro_attribute]
pub fn cb_packet(args: RawToken, st: RawToken) -> RawToken {
    let args = parse_macro_input!(args as CBPacketArgs);
    let st = parse_macro_input!(st as syn::ItemStruct);

    let ident = st.ident.clone();
    let state = args.state;
    let id = args.id;

    let token = quote! {
        #st
        impl ClientBoundPacket for #ident {
            const PACKET_ID: i32 = #id;
            const VALID_STATE: State = #state;
        }
    };

    token.into()
}
