//! Macros for use with Madsim

mod request;
mod service;

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(Request, attributes(rtype))]
pub fn message_derive_rtype(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    request::expand(&ast).into()
}

#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    service::service(args, input)
}

#[allow(clippy::needless_doctest_main)]
/// Marks async function to be executed by the selected runtime. This macro
/// helps set up a `Runtime` without requiring the user to use
/// [Runtime](../madsim/struct.Runtime.html) directly.
///
/// # Example
///
/// ```ignore
/// #[madsim::main]
/// async fn main() {
///     println!("Hello world");
/// }
/// ```
///
/// Equivalent code not using `#[madsim::main]`
///
/// ```ignore
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

    parse_main(input, args).unwrap_or_else(|e| e.to_compile_error().into())
}

fn parse_main(
    mut input: syn::ItemFn,
    _args: syn::AttributeArgs,
) -> Result<TokenStream, syn::Error> {
    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }

    let body = &input.block;
    let brace_token = input.block.brace_token;
    input.block = syn::parse2(quote! {
        {
            let mut rt = madsim::Runtime::new();
            rt.block_on(async #body)
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    let result = quote! {
        #input
    };

    Ok(result.into())
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
///
/// # Configuration
///
/// Test can be configured using the following environment variables:
///
/// - `MADSIM_TEST_SEED`: Set the random seed for test.
///
///     By default, the seed is set to the seconds since the Unix epoch.
///
/// - `MADSIM_TEST_NUM`: Set the number of tests.
///
///     The seed will increase by 1 for each test.
///
///     By default, the number is 1.
///
/// - `MADSIM_TEST_TIME_LIMIT`: Set the time limit for the test.
///
///     The test will panic if time limit exceeded in the simulation.
///
///     By default, there is no time limit.
///
/// - `MADSIM_TEST_CHECK_DETERMINISTIC`: Enable deterministic check.
///
///     The test will be run at least twice with the same seed.
///     If any non-deterministic detected, it will panic as soon as possible.
///
///     By default, it is disabled.
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);

    parse_test(input, args).unwrap_or_else(|e| e.to_compile_error().into())
}

fn parse_test(
    mut input: syn::ItemFn,
    _args: syn::AttributeArgs,
) -> Result<TokenStream, syn::Error> {
    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return Err(syn::Error::new_spanned(input.sig.fn_token, msg));
    }

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
            let mut count: u64 = if let Ok(num_str) = std::env::var("MADSIM_TEST_NUM") {
                num_str.parse().expect("MADSIM_TEST_NUM should be an integer")
            } else {
                1
            };
            let time_limit_s = std::env::var("MADSIM_TEST_TIME_LIMIT").ok().map(|num_str| {
                num_str.parse::<f64>().expect("MADSIM_TEST_TIME_LIMIT should be an number")
            });
            let check = std::env::var("MADSIM_TEST_CHECK_DETERMINISTIC").is_ok();
            if check {
                count = count.max(2);
            }
            let mut rand_log = None;
            for i in 0..count {
                let seed = if check { seed } else { seed + i };
                let rand_log0 = rand_log.take();
                let ret = std::panic::catch_unwind(move || {
                    let mut rt = madsim::Runtime::new_with_seed(seed);
                    if check {
                        rt.enable_deterministic_check(rand_log0);
                    }
                    if let Some(limit) = time_limit_s {
                        rt.set_time_limit(Duration::from_secs_f64(limit));
                    }
                    rt.block_on(async #body);
                    rt.take_rand_log()
                });
                if let Err(e) = ret {
                    println!("MADSIM_TEST_SEED={}", seed);
                    std::panic::resume_unwind(e);
                }
                rand_log = ret.unwrap();
            }
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    let result = quote! {
        #[::core::prelude::v1::test]
        #input
    };

    Ok(result.into())
}
