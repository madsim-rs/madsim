use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token::Comma,
    Attribute, FnArg, Pat, PatType, ReturnType, Token, Visibility,
};

pub struct Service {
    vis: Visibility,
    ident: Ident,
    rpcs: Vec<RpcMethod>,
}

struct RpcMethod {
    ident: Ident,
    input: Option<PatType>,
    output: ReturnType,
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![mod]>()?;
        let ident: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let mut rpcs = Vec::<RpcMethod>::new();
        while !content.is_empty() {
            rpcs.push(content.parse()?);
        }

        Ok(Self { vis, ident, rpcs })
    }
}

impl Parse for RpcMethod {
    fn parse(parse_input: ParseStream) -> syn::Result<Self> {
        let _attrs = parse_input.call(Attribute::parse_outer)?;
        parse_input.parse::<Token![pub]>()?;
        parse_input.parse::<Token![fn]>()?;
        let ident: Ident = parse_input.parse()?;
        let content;
        parenthesized!(content in parse_input);
        let mut input: Option<PatType> = None;

        for arg in content.parse_terminated::<FnArg, Comma>(FnArg::parse)? {
            if input.is_some() {
                return Err(syn::Error::new(
                    arg.span(),
                    "should have at most one argument",
                ));
            };
            match arg {
                FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => {
                    input = Some(captured);
                }
                FnArg::Typed(captured) => {
                    return Err(syn::Error::new(
                        captured.pat.span(),
                        "patterns aren't allowed in RPC args",
                    ));
                }
                FnArg::Receiver(_) => {
                    return Err(syn::Error::new(
                        arg.span(),
                        "method args cannot start with self",
                    ));
                }
            }
        }
        // parse_input.parse::<Token![->]>()?;
        let output = parse_input.parse()?;
        parse_input.parse::<Token![;]>()?;

        Ok(Self {
            ident,
            input,
            output,
        })
    }
}

// generate Rpc tag based on service name and rpc name.
fn gen_id(service: &str, rpc: &str) -> u64 {
    let s = format!("{}:{}", service, rpc);
    let mut h = 0u64;
    let s = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        h = h.wrapping_mul(33).wrapping_add(s[i] as u64);
        i += 1;
    }
    h
}

impl Service {
    fn generate(&self) -> TokenStream2 {
        let Self { vis, ident, rpcs } = self;

        // generate Rpc tag.
        let mut tags = vec![];
        for rpc in rpcs {
            let upper_ident = rpc.ident.to_string().to_uppercase();
            let (tag_var, tag) = (
                format_ident!("{}", upper_ident),
                gen_id(&ident.to_string(), &rpc.ident.to_string()),
            );
            tags.push(quote! {
                pub const #tag_var : u64 = #tag;
            });
        }

        // generate Rpc function
        let mut methods = vec![];
        let unit_ty = &Box::new(parse_quote!(()));
        for rpc in rpcs {
            let ident = &rpc.ident;
            let tag = format_ident!("{}", ident.to_string().to_uppercase());
            let rt_type = match &rpc.output {
                ReturnType::Default => unit_ty,
                ReturnType::Type(_, ty) => ty,
            };
            let input = &rpc.input;
            match input {
                Some(input) => {
                    let arg = match &*input.pat {
                        Pat::Ident(ident) => &ident.ident,
                        _ => unreachable!(),
                    };
                    let arg_ty = &*input.ty;
                    methods.push(quote! {
                        pub fn #ident<'a>(#arg: & #arg_ty) -> crate::net::rpc::RpcRequest<'a, #rt_type, #tag> {
                            crate::net::rpc::RpcRequest::new(#arg)
                        }
                    })
                }
                None => methods.push(quote! {
                    pub fn #ident<'a>() -> crate::net::rpc::RpcRequest<'a, (), #tag> {
                        crate::net::rpc::RpcRequest::new(&())
                    }
                }),
            }
        }

        quote! {
            /// RPC Service: #idnet
            #vis mod #ident {
                #( #tags )*

                #( #methods )*
            }
        }
    }
}

impl ToTokens for Service {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(vec![self.generate()])
    }
}
