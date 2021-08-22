//! Macros for use with Madsim

use proc_macro::TokenStream;
use quote::quote;

#[allow(clippy::needless_doctest_main)]
/// Marks async function to be executed by the selected runtime. This macro
/// helps set up a `Runtime` without requiring the user to use
/// [Runtime](../madsim/struct.Runtime.html) directly.
///
/// # Example
///
/// ```
/// #[madsim::main]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[madsim::main]`
///
/// ```
/// fn main() {
///     madsim::Runtime::new().block_on(async {
///         println!("Hello world");
///     });
/// }
/// ```
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    parse(input, args, false).unwrap_or_else(|e| e.to_compile_error().into())
}

/// Marks async function to be executed by runtime, suitable to test environment.
///
/// # Example
/// ```no_run
/// #[madsim::test]
/// async fn my_test() {
///     assert!(true);
/// }
/// ```
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
            use std::time::{Duration, SystemTime};
            let seed: u64 = if let Ok(seed_str) = std::env::var("MADSIM_TEST_SEED") {
                seed_str.parse().expect("MADSIM_TEST_SEED should be an integer")
            } else {
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
            };
            let count: u64 = if let Ok(num_str) = std::env::var("MADSIM_TEST_NUM") {
                num_str.parse().expect("MADSIM_TEST_NUM should be an integer")
            } else {
                1
            };
            let time_limit_s: u64 = if let Ok(num_str) = std::env::var("MADSIM_TEST_TIME_LIMIT") {
                num_str.parse().expect("MADSIM_TEST_TIME_LIMIT should be an integer")
            } else {
                300
            };
            for i in 0..count {
                let seed = seed + i;
                let ret = std::panic::catch_unwind(|| {
                    let mut rt = madsim::Runtime::new_with_seed(seed);
                    rt.set_time_limit(Duration::from_secs(time_limit_s));
                    rt.block_on(async #body);
                });
                if let Err(e) = ret {
                    println!("seed={}", seed);
                    std::panic::resume_unwind(e);
                }
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
