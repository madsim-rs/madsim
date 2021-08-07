//! Macros for use with Madsim

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    parse(input, args, false).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    parse(input, args, true).unwrap_or_else(|e| e.to_compile_error().into())
}

fn parse(
    mut input: syn::ItemFn,
    _args: syn::AttributeArgs,
    is_test: bool,
) -> Result<TokenStream, syn::Error> {
    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }

    let header = if is_test {
        quote! {
            #[::core::prelude::v1::test]
        }
    } else {
        quote! {}
    };

    let body = &input.block;
    let brace_token = input.block.brace_token;
    input.block = syn::parse2(quote! {
        {
            use std::time::SystemTime;
            let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
            let ret = std::panic::catch_unwind(|| {
                let rt = madsim::Runtime::new_with_seed(seed);
                rt.block_on(async #body);
            });
            if let Err(e) = ret {
                println!("seed={}", seed);
                std::panic::resume_unwind(e);
            }
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    let result = quote! {
        #header
        #input
    };

    Ok(result.into())
}
