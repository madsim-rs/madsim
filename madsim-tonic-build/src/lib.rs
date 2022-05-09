pub mod client;
pub mod server;

pub use tonic_build::{Attributes, Method, Service};

/// Prost generator
#[cfg(feature = "prost")]
#[cfg_attr(docsrs, doc(cfg(feature = "prost")))]
mod prost;

#[cfg(feature = "prost")]
#[cfg_attr(docsrs, doc(cfg(feature = "prost")))]
pub use prost::{compile_protos, configure, Builder};

fn naive_snake_case(name: &str) -> String {
    let mut s = String::new();
    let mut it = name.chars().peekable();

    while let Some(x) = it.next() {
        s.push(x.to_ascii_lowercase());
        if let Some(y) = it.peek() {
            if y.is_uppercase() {
                s.push('_');
            }
        }
    }

    s
}
