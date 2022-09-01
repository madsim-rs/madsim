use darling::FromMeta;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use std::convert::TryFrom;
use syn::{spanned::Spanned, *};

pub fn service(_args: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let mut input = parse_macro_input!(input as ItemImpl);
    service2(&mut input)
        .unwrap_or_else(|e| {
            let ce = e.into_compile_error();
            quote! { #input #ce }
        })
        .into()
}

fn service2(input: &mut ItemImpl) -> Result<TokenStream> {
    let calls = take_rpc_attributes(input)?;
    gen_add_rpc_handler(input, &calls);
    Ok(quote! { #input })
}

/// Find and remove `#[rpc]` attributes.
fn take_rpc_attributes(input: &mut ItemImpl) -> Result<Vec<RpcFn>> {
    #[derive(Debug, Default, FromMeta)]
    #[darling(default)]
    struct RpcArgs {
        read: bool,
        write: bool,
    }

    let mut fns = vec![];
    for item in &mut input.items {
        let method = match item {
            ImplItem::Method(m) => m,
            _ => continue,
        };
        let rpc_meta = match take_attribute(&mut method.attrs, "rpc") {
            Some(v) => v.parse_meta().unwrap(),
            _ => continue,
        };
        let mut rpc_fn = RpcFn::try_from(&method.sig)?;
        if let Meta::List(_) = rpc_meta {
            let args = RpcArgs::from_meta(&rpc_meta).unwrap();
            if args.read && args.write {
                return Err(Error::new(
                    rpc_meta.span(),
                    "can not be both read and write",
                ));
            }
            rpc_fn.read = args.read;
            rpc_fn.write = args.write;
        }
        fns.push(rpc_fn);
    }
    Ok(fns)
}

/// Generate `add_rpc_handler` function.
fn gen_add_rpc_handler(input: &mut ItemImpl, calls: &[RpcFn]) {
    let bodys = calls.iter().map(|f| {
        let name = &f.name;
        let await_suffix = if f.is_async { quote!(.await) } else { quote!() };
        let rpc_type = &f.rpc_type;
        if f.write {
            quote! {
                let this = self.clone();
                ep.add_rpc_handler_with_data(move |req: #rpc_type, data| {
                    let this = this.clone();
                    async move {
                        let ret = this.#name(req, &data)#await_suffix;
                        (ret, vec![])
                    }
                });
            }
        } else if f.read {
            quote! {
                let this = self.clone();
                ep.add_rpc_handler_with_data(move |req: #rpc_type, _| {
                    let this = this.clone();
                    async move { this.#name(req)#await_suffix }
                });
            }
        } else {
            quote! {
                let this = self.clone();
                ep.add_rpc_handler(move |req: #rpc_type| {
                    let this = this.clone();
                    async move { this.#name(req)#await_suffix }
                });
            }
        }
    });
    let serve = quote! {
        pub async fn serve(self, addr: std::net::SocketAddr) -> std::io::Result<()> {
            let ep = madsim::net::Endpoint::bind(addr).await?;
            self.serve_on(ep).await
        }
    };
    let serve_on = quote! {
        pub async fn serve_on(self, ep: madsim::net::Endpoint) -> std::io::Result<()> {
            #(#bodys)*
            madsim::export::futures::future::pending::<()>().await;
            Ok(())
        }
    };
    input.items.push(syn::parse2(serve).unwrap());
    input.items.push(syn::parse2(serve_on).unwrap());
}

/// Find and remove attribute with specific `path`.
fn take_attribute(attrs: &mut Vec<Attribute>, path: &str) -> Option<Attribute> {
    attrs
        .iter()
        .position(|attr| {
            attr.path
                .get_ident()
                .map(|ident| ident == path)
                .unwrap_or(false)
        })
        .map(|idx| attrs.remove(idx))
}

/// Useful information of an RPC function.
struct RpcFn {
    name: Ident,
    is_async: bool,
    rpc_type: Type,
    read: bool,
    write: bool,
}

impl TryFrom<&Signature> for RpcFn {
    type Error = Error;
    fn try_from(sig: &Signature) -> Result<Self> {
        // TODO: assert inputs[0] is &self
        if sig.inputs.len() < 2 {
            return Err(Error::new(sig.inputs.span(), "expect at least 2 arguments"));
        }
        Ok(RpcFn {
            name: sig.ident.clone(),
            is_async: sig.asyncness.is_some(),
            rpc_type: match &sig.inputs[1] {
                FnArg::Typed(pat) => (*pat.ty).clone(),
                _ => panic!("invalid argument"),
            },
            read: false,
            write: false,
        })
    }
}
