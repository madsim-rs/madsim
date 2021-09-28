// Copyright (c) 2017 Actix Team
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};

pub const MESSAGE_ATTR: &str = "rtype";

pub fn expand(ast: &syn::DeriveInput) -> TokenStream {
    let item_type = {
        match get_attribute_type_multiple(ast, MESSAGE_ATTR) {
            Ok(ty) => match ty.len() {
                1 => ty[0].clone(),
                _ => {
                    return syn::Error::new(
                        Span::call_site(),
                        format!(
                            "#[{}(type)] takes 1 parameters, given {}",
                            MESSAGE_ATTR,
                            ty.len()
                        ),
                    )
                    .to_compile_error()
                }
            },
            Err(err) => return err.to_compile_error(),
        }
    };

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let item_type = item_type
        .map(ToTokens::into_token_stream)
        .unwrap_or_else(|| quote! { () });

    quote! {
        impl #impl_generics ::madsim::net::rpc::Request for #name #ty_generics #where_clause {
            type Response = #item_type;
            const ID: u64 = ::madsim::net::rpc::hash_str(concat!(module_path!(), stringify!(#name)));
        }
    }
}

fn get_attribute_type_multiple(
    ast: &syn::DeriveInput,
    name: &str,
) -> syn::Result<Vec<Option<syn::Type>>> {
    let attr = ast
        .attrs
        .iter()
        .find_map(|a| {
            let a = a.parse_meta();
            match a {
                Ok(meta) => {
                    if meta.path().is_ident(name) {
                        Some(meta)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), format!("Expect an attribute `{}`", name))
        })?;

    if let syn::Meta::List(ref list) = attr {
        Ok(list
            .nested
            .iter()
            .map(|m| meta_item_to_ty(m).ok())
            .collect())
    } else {
        Err(syn::Error::new_spanned(
            attr,
            format!("The correct syntax is #[{}(type, type, ...)]", name),
        ))
    }
}

fn meta_item_to_ty(meta_item: &syn::NestedMeta) -> syn::Result<syn::Type> {
    match meta_item {
        syn::NestedMeta::Meta(syn::Meta::Path(ref path)) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map_err(|_| syn::Error::new_spanned(ident, "Expect type")),
            None => Err(syn::Error::new_spanned(path, "Expect type")),
        },
        syn::NestedMeta::Meta(syn::Meta::NameValue(val)) => match val.path.get_ident() {
            Some(ident) if ident == "result" => {
                if let syn::Lit::Str(ref s) = val.lit {
                    if let Ok(ty) = syn::parse_str::<syn::Type>(&s.value()) {
                        return Ok(ty);
                    }
                }
                Err(syn::Error::new_spanned(&val.lit, "Expect type"))
            }
            _ => Err(syn::Error::new_spanned(
                &val.lit,
                r#"Expect `result = "TYPE"`"#,
            )),
        },
        syn::NestedMeta::Lit(syn::Lit::Str(ref s)) => syn::parse_str::<syn::Type>(&s.value())
            .map_err(|_| syn::Error::new_spanned(s, "Expect type")),

        meta => Err(syn::Error::new_spanned(meta, "Expect type")),
    }
}
