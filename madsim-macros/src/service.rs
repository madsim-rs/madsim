use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

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
    let calls = take_rpc_attributes(input);
    gen_add_rpc_handler(input, &calls);
    Ok(quote! { #input })
}

/// Find and remove `#[rpc]` attributes.
fn take_rpc_attributes(input: &mut ItemImpl) -> Vec<RpcFn> {
    let mut fns = Vec::new();
    for item in &mut input.items {
        let method = match item {
            ImplItem::Method(m) => m,
            _ => continue,
        };
        let _call_meta = match take_attribute(&mut method.attrs, "rpc") {
            Some(v) => v.parse_meta().unwrap(),
            _ => continue,
        };
        let fn_info = RpcFn::from(&method.sig);
        fns.push(fn_info);
    }
    fns
}

/// Generate `add_rpc_handler` function.
fn gen_add_rpc_handler(input: &mut ItemImpl, calls: &[RpcFn]) {
    let bodys = calls.iter().map(|f| {
        let name = &f.name;
        let await_suffix = if f.is_async { quote!(.await) } else { quote!() };
        let rpc_type = &f.rpc_type;
        quote! {
            let this = self.clone();
            net.add_rpc_handler(move |req: #rpc_type| {
                let this = this.clone();
                async move { this.#name(req)#await_suffix }
            });
        }
    });
    let add_rpc_handler = quote! {
        fn add_rpc_handler(&self) {
            let net = madsim::net::NetLocalHandle::current();
            #(#bodys)*
        }
    };
    input.items.push(syn::parse2(add_rpc_handler).unwrap());
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
}

impl From<&Signature> for RpcFn {
    fn from(sig: &Signature) -> Self {
        // TODO: assert inputs[0] is &self
        assert_eq!(sig.inputs.len(), 2);
        RpcFn {
            name: sig.ident.clone(),
            is_async: sig.asyncness.is_some(),
            rpc_type: match &sig.inputs[1] {
                FnArg::Typed(pat) => (*pat.ty).clone(),
                _ => panic!("invalid argument"),
            },
        }
    }
}
