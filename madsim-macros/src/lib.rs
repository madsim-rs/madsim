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
/// [Runtime](../madsim/runtime/struct.Runtime.html) directly.
///
/// # Example
///
/// ```ignore
/// #[madsim::main]
/// async fn main() {
///     println!("Hello world");
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
/// ```ignore
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
/// - `MADSIM_TEST_JOBS`: Set the number of jobs to run simultaneously.
///
///     By default, the number of jobs is 1.
///
/// - `MADSIM_TEST_CONFIG`: Set the config file path.
///
///     By default, tests will use the default configuration.
///
/// - `MADSIM_TEST_TIME_LIMIT`: Set the time limit for the test.
///
///     The test will panic if time limit exceeded in the simulation.
///
///     By default, there is no time limit.
///
/// - `MADSIM_TEST_CHECK_DETERMINISM`: Enable determinism check.
///
///     The test will be run at least twice with the same seed.
///     If any non-determinism detected, it will panic as soon as possible.
///
///     By default, it is disabled.
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

    let body = &input.block;
    let brace_token = input.block.brace_token;
    input.block = syn::parse2(quote! {
        {
            ::madsim::runtime::init_logger();
            ::madsim::runtime::Builder::from_env().run(|| async #body)
        }
    })
    .expect("Parsing failure");
    input.block.brace_token = brace_token;

    let attribute = if is_test {
        quote! { #[::core::prelude::v1::test] }
    } else {
        quote! {}
    };

    let result = quote! {
        #attribute
        #input
    };
    Ok(result.into())
}
