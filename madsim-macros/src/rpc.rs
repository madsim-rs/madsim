use convert_case::*;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    Attribute, FnArg, Item, Pat, ReturnType, Signature, Token, Type, Visibility,
};

pub struct Service {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    items: Vec<Item>,
    rpcs: Vec<RpcMethod>,
}

struct RpcMethod {
    vis: Visibility,
    sig: Signature,
    input: Option<FnArg>,
    in_data: Option<Ident>,
    output: ReturnType,
    out_data: Option<Ident>,
}

fn decode_arg(arg: &FnArg) -> syn::Result<(&Ident, &Type)> {
    match arg {
        FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => {
            let ident = match &*captured.pat {
                Pat::Ident(ident) => ident,
                _ => unreachable!(),
            };
            Ok((&ident.ident, &*captured.ty))
        }
        FnArg::Typed(captured) => Err(syn::Error::new(captured.pat.span(), "invalid pattern")),
        FnArg::Receiver(_) => Err(syn::Error::new(
            arg.span(),
            "method args cannot start with self",
        )),
    }
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![mod]>()?;
        let ident: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let mut rpcs = Vec::new();
        let mut items: Vec<Item> = Vec::new();
        while !content.is_empty() {
            let fork = content.fork();
            let item = content.parse::<Item>()?;
            // from item to RPC method
            match &item {
                Item::Verbatim(_) => {
                    let method = fork.parse()?;
                    rpcs.push(method);
                }
                _ => items.push(item),
            }
        }

        Ok(Self {
            attrs,
            vis,
            ident,
            items,
            rpcs,
        })
    }
}

impl Service {
    fn generate(&self) -> TokenStream2 {
        let Self {
            attrs,
            vis,
            ident,
            items,
            rpcs,
        } = self;

        let tokens: Vec<_> = rpcs.iter().map(|rpc| rpc.generate(&ident)).collect();

        let out = quote! {
            /// RPC Service: #idnet
            #( #attrs )* #vis mod #ident {
                // origin items.
                #( #items )*
                // generated tokens.
                #( #tokens )*
            }
        };
        out
    }
}

impl ToTokens for Service {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(vec![self.generate()])
    }
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;
        input.parse::<Token![;]>()?;

        // parse input/output information;
        // decode input/output information;
        let mut input = None;
        let mut in_data = None;
        let mut out_data = None;
        let output = sig.output.clone();
        let in_data_ty: &Type = &parse_quote!(RpcInData);
        let out_data_ty: &Type = &parse_quote!(RpcOutData);
        for arg in sig.inputs.iter() {
            // Arguments should in order: (input, in_data, out_data);
            let (ident, ty) = decode_arg(arg)?;
            if input.is_none()
                && in_data.is_none()
                && out_data.is_none()
                && ty != in_data_ty
                && ty != out_data_ty
            {
                input = Some(arg.to_owned());
            } else if in_data.is_none() && out_data.is_none() && ty == in_data_ty {
                in_data = Some(ident.to_owned());
            } else if out_data.is_none() && ty == out_data_ty {
                out_data = Some(ident.to_owned());
            } else {
                return Err(syn::Error::new(arg.span(), "wrong RPC method format"));
            }
        }

        Ok(Self {
            vis,
            sig,
            input,
            in_data,
            output,
            out_data,
        })
    }
}

impl RpcMethod {
    fn generate(&self, service: &Ident) -> TokenStream2 {
        let mut tokens = vec![];
        // generate RPC id based on RPC service name & method name
        let ident = &self.sig.ident;
        let id = gen_id(&service.to_string(), &ident.to_string());

        // generate RPC type information.
        let vis = &self.vis;
        let generics = &self.sig.generics;
        let type_ident = Ident::new(
            ident.to_string().to_case(Case::Pascal).as_str(),
            ident.span(),
        );
        let unit_ty: &Type = &parse_quote!(());
        let req = self
            .input
            .as_ref()
            .map(|arg| decode_arg(arg).unwrap().1)
            .unwrap_or(unit_ty);
        let req_ident = self.input.as_ref().map(|arg| decode_arg(&arg).unwrap().0);
        let resp = match &self.output {
            ReturnType::Default => unit_ty,
            ReturnType::Type(_, ty) => &*ty,
        };
        tokens.push(quote! {
            #vis struct #type_ident{}
            impl #generics madsim::net::rpc::RpcType<#req, #resp> for #type_ident {
                const ID: u64 = #id;
            }
        });
        if req != unit_ty {
            tokens.push(quote! {
                impl #generics madsim::net::rpc::RpcType<(), #resp> for #type_ident {
                    const ID: u64 = #id;
                }
            });
        }
        tokens.push(match self.in_data {
            Some(_) => quote! {
                impl madsim::net::rpc::RpcInData<true> for #type_ident {}
            },
            None => quote! {
                impl madsim::net::rpc::RpcInData<false> for #type_ident {}
            },
        });
        tokens.push(match self.out_data {
            Some(_) => quote! {
                impl madsim::net::rpc::RpcOutData<true> for #type_ident {}
            },
            None => quote! {
                impl madsim::net::rpc::RpcOutData<false> for #type_ident {}
            },
        });

        // generate RPC client method
        let mut generics = self.sig.generics.clone();
        generics.params.push(parse_quote!('_a));
        let input = &self.input;
        let in_data = &self.in_data;
        match (input, in_data) {
            (Some(_), None) => {
                let req_ident = req_ident.unwrap();
                tokens.push(quote! {
                    pub fn #ident #generics (#req_ident: &'_a #req) ->
                        madsim::net::rpc::RpcRequest<'_a, #req, #resp, #type_ident>
                    {
                        madsim::net::rpc::RpcRequest::new(#req_ident)
                    }
                })
            }
            (Some(_), Some(data)) => {
                let req_ident = req_ident.unwrap();
                tokens.push(quote! {
                    pub fn #ident #generics (#req_ident: &#req, #data: &'_a [u8])
                        -> madsim::net::rpc::RpcRequest<'_a, #req, #resp, #type_ident>
                    {
                        madsim::net::rpc::RpcRequest::new_data(#req_ident, #data)
                    }
                })
            }
            (None, None) => tokens.push(quote! {
                pub fn #ident #generics () ->
                    madsim::net::rpc::RpcRequest<'_a, #req, #resp, #type_ident>
                {
                    madsim::net::rpc::RpcRequest::new(&())
                }
            }),
            (None, Some(data)) => tokens.push(quote! {
                pub fn #ident #generics (#data: &'_a [u8]) ->
                    madsim::net::rpc::RpcRequest<'_a, #req, #resp, #type_ident>
                {
                    madsim::net::rpc::RpcRequest::new_data(&(), #data)
                }
            }),
        }

        quote! {
            #( #tokens )*
        }
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
