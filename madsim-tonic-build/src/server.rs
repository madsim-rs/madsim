use super::{Attributes, Method, Service};
use crate::naive_snake_case;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Lit, LitStr};

/// Generate service for Server.
///
/// This takes some `Service` and will generate a `TokenStream` that contains
/// a public module containing the server service and handler trait.
pub fn generate<T: Service>(
    service: &T,
    emit_package: bool,
    proto_path: &str,
    compile_well_known_types: bool,
    _attributes: &Attributes,
) -> TokenStream {
    let methods = generate_methods(service, proto_path, compile_well_known_types);

    let server_service = quote::format_ident!("{}Server", service.name());
    let server_trait = quote::format_ident!("{}", service.name());
    let server_mod = quote::format_ident!("{}_server", naive_snake_case(service.name()));
    let generated_trait = generate_trait(
        service,
        proto_path,
        compile_well_known_types,
        server_trait.clone(),
    );
    // let service_doc = generate_doc_comments(service.comment());
    let package = if emit_package { service.package() } else { "" };
    // Transport based implementations
    let path = format!(
        "{}{}{}",
        package,
        if package.is_empty() { "" } else { "." },
        service.identifier()
    );
    let transport = generate_transport(&server_service, &server_trait, &path);
    // let mod_attributes = attributes.for_mod(package);
    // let struct_attributes = attributes.for_struct(&path);

    let compression_enabled = cfg!(feature = "compression");

    // let compression_config_ty = if compression_enabled {
    //     quote! { EnabledCompressionEncodings }
    // } else {
    //     quote! { () }
    // };

    let configure_compression_methods = if compression_enabled {
        quote! {
            /// Enable decompressing requests with `gzip`.
            #[must_use]
            pub fn accept_gzip(self) -> Self {
                // self.accept_compression_encodings.enable_gzip();
                self
            }

            /// Compress responses with `gzip`, if the client supports it.
            #[must_use]
            pub fn send_gzip(self) -> Self {
                // self.send_compression_encodings.enable_gzip();
                self
            }
        }
    } else {
        quote! {}
    };

    let configure_max_message_size_methods = quote! {
        /// Limits the maximum size of a decoded message.
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            // self.max_decoding_message_size = Some(limit);
            self
        }

        /// Limits the maximum size of an encoded message.
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            // self.max_encoding_message_size = Some(limit);
            self
        }
    };

    quote! {
        /// Generated server implementations.
        // #(#mod_attributes)*
        pub mod #server_mod {
            #![allow(
                unused_imports,
                unused_variables,
                dead_code,
                missing_docs,
                unused_mut,
                // will trigger if compression is disabled
                clippy::let_unit_value,
            )]
            use tonic::codegen::{http::uri::PathAndQuery, futures::{stream::{self, StreamExt}, future::FutureExt}, *};

            #generated_trait

            // #service_doc
            // #(#struct_attributes)*
            #[derive(Debug)]
            pub struct #server_service<T: #server_trait, F> {
                inner: Arc<T>,
                interceptor: F,
            }

            impl<T: #server_trait> #server_service<T, IdentityInterceptor> {
                pub fn new(inner: T) -> Self {
                    Self::from_arc(Arc::new(inner))
                }

                pub fn from_arc(inner: Arc<T>) -> Self {
                    Self {
                        inner,
                        interceptor: Ok,
                    }
                }
            }

            impl<T: #server_trait, F> #server_service<T, F>
            where
                F: tonic::service::Interceptor,
            {
                pub fn with_interceptor(inner: T, interceptor: F) -> Self {
                    Self {
                        inner: Arc::new(inner),
                        interceptor,
                    }
                }

                #configure_compression_methods
                #configure_max_message_size_methods
            }

            impl<T, F> tonic::codegen::Service<(PathAndQuery, tonic::Request<BoxMessageStream>)> for #server_service<T, F>
            where
                T: #server_trait,
                F: tonic::service::Interceptor,
            {
                type Response = tonic::Response<BoxMessageStream>;
                type Error = tonic::Status;
                type Future = BoxFuture<Self::Response, Self::Error>;

                fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                    Poll::Ready(Ok(()))
                }

                fn call(&mut self, (path, request): (PathAndQuery, tonic::Request<BoxMessageStream>)) -> Self::Future {
                    let inner = self.inner.clone();
                    let mut request = match request.intercept(&mut self.interceptor) {
                        Ok(r) => r,
                        Err(e) => return Box::pin(async move { Err(e) }),
                    };

                    match path.path() {
                        #methods

                        _ => Box::pin(async move { Err(tonic::Status::invalid_argument(format!("no path: {path}"))) }),
                    }
                }
            }

            impl<T, F> Clone for #server_service<T, F>
            where
                T: #server_trait,
                F: Clone,
            {
                fn clone(&self) -> Self {
                    Self {
                        inner: self.inner.clone(),
                        interceptor: self.interceptor.clone(),
                    }
                }
            }

            #transport
        }
    }
}

fn generate_trait<T: Service>(
    service: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    server_trait: Ident,
) -> TokenStream {
    let methods = generate_trait_methods(service, proto_path, compile_well_known_types);
    // let trait_doc = generate_doc_comment(&format!(
    //     "Generated trait containing gRPC methods that should be implemented for use with {}Server.",
    //     service.name()
    // ));

    quote! {
        // #trait_doc
        #[async_trait]
        pub trait #server_trait : Send + Sync + 'static {
            #methods
        }
    }
}

fn generate_trait_methods<T: Service>(
    service: &T,
    proto_path: &str,
    compile_well_known_types: bool,
) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let name = quote::format_ident!("{}", method.name());

        let (req_message, res_message) =
            method.request_response_name(proto_path, compile_well_known_types);

        // let method_doc = generate_doc_comments(method.comment());

        let method = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => {
                quote! {
                    // #method_doc
                    async fn #name(&self, request: tonic::Request<#req_message>)
                        -> Result<tonic::Response<#res_message>, tonic::Status>;
                }
            }
            (true, false) => {
                quote! {
                    // #method_doc
                    async fn #name(&self, request: tonic::Request<tonic::Streaming<#req_message>>)
                        -> Result<tonic::Response<#res_message>, tonic::Status>;
                }
            }
            (false, true) => {
                let stream = quote::format_ident!("{}Stream", method.identifier());
                // let stream_doc = generate_doc_comment(&format!(
                //     "Server streaming response type for the {} method.",
                //     method.identifier()
                // ));

                quote! {
                    // #stream_doc
                    type #stream: futures_core::Stream<Item = Result<#res_message, tonic::Status>> + Send + 'static;

                    // #method_doc
                    async fn #name(&self, request: tonic::Request<#req_message>)
                        -> Result<tonic::Response<Self::#stream>, tonic::Status>;
                }
            }
            (true, true) => {
                let stream = quote::format_ident!("{}Stream", method.identifier());
                // let stream_doc = generate_doc_comment(&format!(
                //     "Server streaming response type for the {} method.",
                //     method.identifier()
                // ));

                quote! {
                    // #stream_doc
                    type #stream: futures_core::Stream<Item = Result<#res_message, tonic::Status>> + Send + 'static;

                    // #method_doc
                    async fn #name(&self, request: tonic::Request<tonic::Streaming<#req_message>>)
                        -> Result<tonic::Response<Self::#stream>, tonic::Status>;
                }
            }
        };

        stream.extend(method);
    }

    stream
}

#[cfg(feature = "transport")]
fn generate_transport(
    server_service: &syn::Ident,
    server_trait: &syn::Ident,
    service_name: &str,
) -> TokenStream {
    let service_name = syn::LitStr::new(service_name, proc_macro2::Span::call_site());

    quote! {
        impl<T, F> tonic::transport::NamedService for #server_service<T, F>
        where
            T: #server_trait,
            F: tonic::service::Interceptor,
        {
            const NAME: &'static str = #service_name;
        }
    }
}

#[cfg(not(feature = "transport"))]
fn generate_transport(
    _server_service: &syn::Ident,
    _server_trait: &syn::Ident,
    _service_name: &str,
) -> TokenStream {
    TokenStream::new()
}

fn generate_methods<T: Service>(
    service: &T,
    proto_path: &str,
    compile_well_known_types: bool,
) -> TokenStream {
    let mut stream = TokenStream::new();

    for method in service.methods() {
        let path = format!(
            "/{}{}{}/{}",
            service.package(),
            if service.package().is_empty() {
                ""
            } else {
                "."
            },
            service.identifier(),
            method.identifier()
        );
        let method_path = Lit::Str(LitStr::new(&path, Span::call_site()));
        let ident = quote::format_ident!("{}", method.name());
        let server_trait = quote::format_ident!("{}", service.name());

        let method_stream = match (method.client_streaming(), method.server_streaming()) {
            (false, false) => generate_unary(
                method,
                proto_path,
                compile_well_known_types,
                ident,
                server_trait,
            ),

            (false, true) => generate_server_streaming(
                method,
                proto_path,
                compile_well_known_types,
                ident.clone(),
                server_trait,
            ),
            (true, false) => generate_client_streaming(
                method,
                proto_path,
                compile_well_known_types,
                ident.clone(),
                server_trait,
            ),

            (true, true) => generate_streaming(
                method,
                proto_path,
                compile_well_known_types,
                ident.clone(),
                server_trait,
            ),
        };

        let method = quote! {
            #method_path => {
                #method_stream
            }
        };
        stream.extend(method);
    }

    stream
}

fn generate_unary<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    method_ident: Ident,
    _server_trait: Ident,
) -> TokenStream {
    let (request, _) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        let inner = self.inner.clone();
        Box::pin(async move {
            let request = request.map(|mut stream| {
                let first = stream.next().now_or_never().unwrap().unwrap();
                *first.unwrap().downcast::<#request>().unwrap()
            });
            let response: tonic::Response<_> = (*inner).#method_ident(request).await?;
            Ok(response.map(|msg| stream::once(async move { Ok(Box::new(msg) as BoxMessage) }).boxed()))
        })
    }
}

fn generate_server_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    method_ident: Ident,
    _server_trait: Ident,
) -> TokenStream {
    let (request, _) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        let inner = self.inner.clone();
        Box::pin(async move {
            let request = request.map(|mut stream| {
                let first = stream.next().now_or_never().unwrap().unwrap();
                *first.unwrap().downcast::<#request>().unwrap()
            });
            let response: tonic::Response<_> = (*inner).#method_ident(request).await?;
            Ok(response.map(|stream| stream.map(|res| res.map(|msg| Box::new(msg) as BoxMessage)).boxed()))
        })
    }
}

fn generate_client_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    method_ident: Ident,
    _server_trait: Ident,
) -> TokenStream {
    let (request, _) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        let inner = self.inner.clone();
        Box::pin(async move {
            let request = request.map(|stream| {
                tonic::Streaming::from_stream(
                    stream.map(|res| res.map(|msg| *msg.downcast::<#request>().unwrap())).boxed()
                )
            });
            let response: tonic::Response<_> = (*inner).#method_ident(request).await?;
            Ok(response.map(|msg| stream::once(async move { Ok(Box::new(msg) as BoxMessage) }).boxed()))
        })
    }
}

fn generate_streaming<T: Method>(
    method: &T,
    proto_path: &str,
    compile_well_known_types: bool,
    method_ident: Ident,
    _server_trait: Ident,
) -> TokenStream {
    let (request, _) = method.request_response_name(proto_path, compile_well_known_types);

    quote! {
        let inner = self.inner.clone();
        Box::pin(async move {
            let request = request.map(|stream| {
                tonic::Streaming::from_stream(
                    stream.map(|res| res.map(|msg| *msg.downcast::<#request>().unwrap())).boxed()
                )
            });
            let response: tonic::Response<_> = (*inner).#method_ident(request).await?;
            Ok(response.map(|stream| stream.map(|res| res.map(|msg| Box::new(msg) as BoxMessage)).boxed()))
        })
    }
}
