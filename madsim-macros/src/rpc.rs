use convert_case::*;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token::Comma,
    Attribute, FnArg, Pat, PatType, ReturnType, Token, Type, Visibility,
};

pub struct Service {
    vis: Visibility,
    ident: Ident,
    rpcs: Vec<RpcMethod>,
}

struct RpcMethod {
    ident: Ident,
    input: Option<PatType>,
    in_data: Option<Ident>,
    output: ReturnType,
    out_data: Option<Ident>,
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
        parse_input.parse::<Token![pub]>().ok();
        parse_input.parse::<Token![fn]>()?;
        let ident: Ident = parse_input.parse()?;
        let content;
        parenthesized!(content in parse_input);
        let mut input: Option<PatType> = None;

        let in_data_ty: &Type = &parse_quote!(RpcInData);
        let out_data_ty: &Type = &parse_quote!(RpcOutData);
        let mut in_data = None;
        let mut out_data = None;

        for arg in content.parse_terminated::<FnArg, Comma>(FnArg::parse)? {
            let captured = match arg {
                FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => captured,
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
            };

            if captured.ty.as_ref() == in_data_ty {
                if in_data.is_some() {
                    return Err(syn::Error::new(
                        captured.pat.span(),
                        "at most one RpcInData",
                    ));
                }
                let ident = match &*captured.pat {
                    Pat::Ident(ident) => ident.ident.clone(),
                    _ => {
                        return Err(syn::Error::new(
                            captured.pat.span(),
                            "patterns aren't allowed in RPC args",
                        ))
                    }
                };
                in_data = Some(ident);
                continue;
            }

            if captured.ty.as_ref() == out_data_ty {
                if out_data.is_some() {
                    return Err(syn::Error::new(
                        captured.pat.span(),
                        "at most one RpcOutData",
                    ));
                }
                let ident = match &*captured.pat {
                    Pat::Ident(ident) => ident.ident.clone(),
                    _ => {
                        return Err(syn::Error::new(
                            captured.pat.span(),
                            "patterns aren't allowed in RPC args",
                        ))
                    }
                };
                out_data = Some(ident);
                continue;
            }

            if input.is_some() {
                return Err(syn::Error::new(
                    captured.span(),
                    "should have at most one argument",
                ));
            };
            input = Some(captured);
        }
        // parse_input.parse::<Token![->]>()?;
        let output = parse_input.parse()?;
        parse_input.parse::<Token![;]>()?;

        Ok(Self {
            ident,
            input,
            in_data,
            output,
            out_data,
        })
    }
}

// generate RPC tag based on service name and rpc name.
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
        let service_name = ident.to_string();

        let unit_ty: &Type = &parse_quote!(());

        // generate RpcType.
        let mut tokens = vec![];
        for rpc in rpcs {
            let ident = rpc.ident.clone();
            let rpc_name = ident.to_string();
            let id = gen_id(&service_name, &rpc_name);
            let rpc_ident = format_ident!("{}", rpc_name.to_case(Case::Pascal));
            let req = rpc.input.as_ref().map(|t| t.ty.as_ref()).unwrap_or(unit_ty);
            let rsp = match &rpc.output {
                ReturnType::Default => unit_ty,
                ReturnType::Type(_, ty) => ty,
            };
            tokens.push(quote! {
                pub struct #rpc_ident();
                impl crate::net::rpc::RpcType<#req, #rsp> for #rpc_ident {
                    const ID: u64 = #id;
                }
            });
            if rpc.input.is_some() {
                tokens.push(quote! {
                    impl crate::net::rpc::RpcType<(), #rsp> for #rpc_ident {
                        const ID: u64 = #id;
                    }
                })
            }
            tokens.push(match rpc.in_data {
                Some(_) => quote! {
                    impl crate::net::rpc::RpcInData<true> for #rpc_ident {}
                },
                None => quote! {
                    impl crate::net::rpc::RpcInData<false> for #rpc_ident {}
                },
            });
            tokens.push(match rpc.out_data {
                Some(_) => quote! {
                    impl crate::net::rpc::RpcOutData<true> for #rpc_ident {}
                },
                None => quote! {
                    impl crate::net::rpc::RpcOutData<false> for #rpc_ident {}
                },
            });

            let input = &rpc.input;
            let in_data = &rpc.in_data;
            match (input, in_data) {
                (Some(req_pat), None) => {
                    let req_ident = match &*req_pat.pat {
                        Pat::Ident(ident) => &ident.ident,
                        _ => unreachable!(),
                    };
                    tokens.push(
                        quote! {
                            pub fn #ident<'a>(#req_ident: &'a #req) -> crate::net::rpc::RpcRequest<'a, #rsp, #rpc_ident> {
                                crate::net::rpc::RpcRequest::new(#req_ident)
                            }
                        }
                    )
                },
                (Some(req_pat), Some(data)) => {
                    let req_ident = match &*req_pat.pat {
                        Pat::Ident(ident) => &ident.ident,
                        _ => unreachable!(),
                    };
                    tokens.push(
                        quote! {
                            pub fn #ident<'a, 'b>(#req_ident: &'b #req, #data: &'a [u8]) -> crate::net::rpc::RpcRequest<'a, #rsp, #rpc_ident> {
                                crate::net::rpc::RpcRequest::new_data(#req_ident, #data)
                            }
                        }
                    )
                },
                (None, None) => {
                    tokens.push(
                        quote! {
                            pub fn #ident<'a>() -> crate::net::rpc::RpcRequest<'a, #rsp, #rpc_ident> {
                                crate::net::rpc::RpcRequest::new(&())
                            }
                        }
                    )
                },
                (None, Some(data)) => {
                    tokens.push(
                        quote! {
                            pub fn #ident<'a>(#data: &'a [u8]) -> crate::net::rpc::RpcRequest<'a, #rsp, #rpc_ident> {
                                crate::net::rpc::RpcRequest::new_data(&(), #data)
                            }
                        }
                    )
                },
            }
        }

        quote! {
            /// RPC Service: #idnet
            #vis mod #ident {
                #( #tokens )*
            }
        }
    }
}

impl ToTokens for Service {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(vec![self.generate()])
    }
}
