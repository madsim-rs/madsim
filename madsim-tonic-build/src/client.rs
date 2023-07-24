use super::{Attributes, Method, Service};
use crate::naive_snake_case;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate service for client.
///
/// This takes some `Service` and will generate a `TokenStream` that contains
/// a public module with the generated client.
pub fn generate<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    _attributes: &Attributes,
) -> TokenStream {
    let service_ident = quote::format_ident!("{}Client", service.name());
    let client_mod = quote::format_ident!("{}_client", naive_snake_case(service.name()));
    let methods = generate_methods(service, emit_package, proto_path, compile_well_known_types);

    let connect = generate_connect(&service_ident);
    // let service_doc = generate_doc_comments(service.comment());

    // let package = if emit_package { service.package() } else { "" };
    // let path = format!(
    //     "{}{}{}",
    //     package,
    //     if package.is_empty() { "" } else { "." },
    //     service.identifier()
    // );

    // let mod_attributes = attributes.for_mod(package);
    // let struct_attributes = attributes.for_struct(&path);

    quote! {
        /// Generated client implementations.
        // #(#mod_attributes)*
        pub mod #client_mod {
            #![allow(
                unused_variables,
                dead_code,
                missing_docs,
                // will trigger if compression is disabled
                clippy::let_unit_value,
            )]
            use tonic::codegen::*;

            // #service_doc
            // #(#struct_attributes)*
            #[derive(Debug, Clone)]
            pub struct #service_ident<T, F = IdentityInterceptor> {
                inner: tonic::client::Grpc<T, F>,
            }

            #connect

            impl #service_ident<tonic::transport::Channel> {
                pub fn new(inner: tonic::transport::Channel) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    Self { inner }
                }
            }

            impl<F: tonic::service::Interceptor> #service_ident<tonic::transport::Channel, F> {
                pub fn with_interceptor(inner: tonic::transport::Channel, interceptor: F)
                    -> #service_ident<tonic::transport::Channel, F>
                {
                    let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
                    Self { inner }
                }

                /// Compress requests with the given encoding.
                ///
                /// This requires the server to support it otherwise it might respond with an
                /// error.
                #[must_use]
                pub fn send_compressed(self, encoding: CompressionEncoding) -> Self {
                    // self.inner = self.inner.send_compressed(encoding);
                    self
                }
                /// Enable decompressing responses.
                #[must_use]
                pub fn accept_compressed(self, encoding: CompressionEncoding) -> Self {
                    // self.inner = self.inner.accept_compressed(encoding);
                    self
                }
                /// Limits the maximum size of a decoded message.
                #[must_use]
                pub fn max_decoding_message_size(self, limit: usize) -> Self {
                    // self.inner = self.inner.max_decoding_message_size(limit);
                    self
                }

                /// Limits the maximum size of an encoded message.
                #[must_use]
                pub fn max_encoding_message_size(self, limit: usize) -> Self {
                    // self.inner = self.inner.max_encoding_message_size(limit);
                    self
                }

                #methods
            }
        }
    }
}

#[cfg(feature = "transport")]
fn generate_connect(service_ident: &syn::Ident) -> TokenStream {
    quote! {
        impl #service_ident<tonic::transport::Channel> {
            /// Attempt to create a new client by connecting to a given endpoint.
            pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
            where
                D: std::convert::TryInto<tonic::transport::Endpoint>,
                D::Error: Into<StdError>,
            {
                let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                Ok(Self::new(conn))
            }
        }
    }
}

#[cfg(not(feature = "transport"))]
fn generate_connect(_service_ident: &syn::Ident) -> TokenStream {
    TokenStream::new()
}

fn generate_methods<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
) -> TokenStream {
    let mut stream = TokenStream::new();
    let package = if emit_package { service.package() } else { "" };

    for method in service.methods() {
        let path = format!(
            "/{}{}{}/{}",
            package,
            if package.is_empty() { "" } else { "." },
            service.identifier(),
            method.identifier()
        );

        // stream.extend(generate_doc_comments(method.comment()));

        let method = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => generate_unary(method, proto_path, compile_well_known_types, path),
            (false, true) => {
                generate_server_streaming(method, proto_path, compile_well_known_types, path)
            }
            (true, false) => {
                generate_client_streaming(method, proto_path, compile_well_known_types, path)
            }
            (true, true) => generate_streaming(method, proto_path, compile_well_known_types, path),
        };

        stream.extend(method);
    }

    stream
}

fn generate_unary<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    path: String,
) -> TokenStream {
    // let codec_name = syn::parse_str::<syn::Path>(method.codec_path()).unwrap();
    let ident = format_ident!("{}", method.name());
    let (request, response) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        pub async fn #ident(
            &mut self,
            request: impl tonic::IntoRequest<#request>,
        ) -> Result<tonic::Response<#response>, tonic::Status> {
           self.inner.ready().await.map_err(|e| {
               tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {e}"))
           })?;
           // let codec = #codec_name::default();
           let codec = ();
           let path = http::uri::PathAndQuery::from_static(#path);
           self.inner.unary(request.into_request(), path, codec).await
        }
    }
}

fn generate_server_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    path: String,
) -> TokenStream {
    // let codec_name = syn::parse_str::<syn::Path>(method.codec_path()).unwrap();
    let ident = format_ident!("{}", method.name());

    let (request, response) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        pub async fn #ident(
            &mut self,
            request: impl tonic::IntoRequest<#request>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<#response>>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {e}"))
            })?;
            // let codec = #codec_name::default();
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(#path);
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
    }
}

fn generate_client_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    path: String,
) -> TokenStream {
    // let codec_name = syn::parse_str::<syn::Path>(method.codec_path()).unwrap();
    let ident = format_ident!("{}", method.name());

    let (request, response) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        pub async fn #ident(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = #request>
        ) -> Result<tonic::Response<#response>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {e}"))
            })?;
            // let codec = #codec_name::default();
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(#path);
            self.inner.client_streaming(request.into_streaming_request(), path, codec).await
        }
    }
}

fn generate_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    path: String,
) -> TokenStream {
    // let codec_name = syn::parse_str::<syn::Path>(method.codec_path()).unwrap();
    let ident = format_ident!("{}", method.name());

    let (request, response) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        pub async fn #ident(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = #request>
        ) -> Result<tonic::Response<tonic::codec::Streaming<#response>>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {e}"))
            })?;
            // let codec = #codec_name::default();
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(#path);
            self.inner.streaming(request.into_streaming_request(), path, codec).await
        }
    }
}
