#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloRequest {
    /// Request message contains the name to be greeted
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloReply {
    /// Reply contains the greeting message
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
}
/// Generated client implementations.
pub mod greeter_client {
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct GreeterClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl GreeterClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }

        pub fn new(inner: tonic::transport::Channel) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        // missing methods:
        // - pub fn with_interceptor<F>()
        // - pub fn send_gzip(mut self) -> Self
        // - pub fn accept_gzip(mut self) -> Self

        /// Our SayHello rpc accepts HelloRequests and returns HelloReplies
        pub async fn say_hello(
            &mut self,
            request: impl tonic::IntoRequest<super::HelloRequest>,
        ) -> Result<tonic::Response<super::HelloReply>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e),
                )
            })?;
            let codec = ();
            let path = http::uri::PathAndQuery::from_static("/helloworld.Greeter/SayHello");
            self.inner
                .unary(tonic::Request::new(request), path, codec)
                .await
        }
        pub async fn lots_of_replies(
            &mut self,
            request: impl tonic::IntoRequest<super::HelloRequest>,
        ) -> Result<
                tonic::Response<tonic::codec::Streaming<super::HelloReply>>,
                tonic::Status,
            > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e),
                    )
                })?;
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(
                "/helloworld.Greeter/LotsOfReplies",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        pub async fn lots_of_greetings(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::HelloRequest>,
        ) -> Result<tonic::Response<super::HelloReply>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e),
                    )
                })?;
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(
                "/helloworld.Greeter/LotsOfGreetings",
            );
            self.inner
                .client_streaming(request.into_streaming_request(), path, codec)
                .await
        }
        pub async fn bidi_hello(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::HelloRequest>,
        ) -> Result<
                tonic::Response<tonic::codec::Streaming<super::HelloReply>>,
                tonic::Status,
            > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e),
                    )
                })?;
            let codec = ();
            let path = http::uri::PathAndQuery::from_static(
                "/helloworld.Greeter/BidiHello",
            );
            self.inner.streaming(request.into_streaming_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod greeter_server {
    use tonic::codegen::{http::uri::PathAndQuery, *};
    type BoxMessage = Box<dyn std::any::Any + Send + Sync>;

    ///Generated trait containing gRPC methods that should be implemented for use with GreeterServer.
    #[async_trait]
    pub trait Greeter: Send + Sync + 'static {
        /// Our SayHello rpc accepts HelloRequests and returns HelloReplies
        async fn say_hello(
            &self,
            request: tonic::Request<super::HelloRequest>,
        ) -> Result<tonic::Response<super::HelloReply>, tonic::Status>;

        type LotsOfRepliesStream: futures_core::Stream<Item = Result<super::HelloReply, tonic::Status>>
            + Send
            + 'static;
        async fn lots_of_replies(
            &self,
            request: tonic::Request<super::HelloRequest>,
        ) -> Result<tonic::Response<Self::LotsOfRepliesStream>, tonic::Status>;
        async fn lots_of_greetings(
            &self,
            request: tonic::Request<tonic::Streaming<super::HelloRequest>>,
        ) -> Result<tonic::Response<super::HelloReply>, tonic::Status>;
        ///Server streaming response type for the BidiHello method.
        type BidiHelloStream: futures_core::Stream<Item = Result<super::HelloReply, tonic::Status>>
            + Send
            + 'static;
        async fn bidi_hello(
            &self,
            request: tonic::Request<tonic::Streaming<super::HelloRequest>>,
        ) -> Result<tonic::Response<Self::BidiHelloStream>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct GreeterServer<T: Greeter> {
        inner: Arc<T>,
    }
    impl<T: Greeter> GreeterServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self { inner }
        }
        // pub fn with_interceptor<F>(inner: T, interceptor: F)
    }
    impl<T: Greeter> tonic::codegen::Service<(PathAndQuery, BoxMessage)> for GreeterServer<T> {
        type Response = BoxMessage;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, (path, req): (PathAndQuery, BoxMessage)) -> Self::Future {
            match path.path() {
                "/helloworld.Greeter/SayHello" => {
                    let inner = self.inner.clone();
                    Box::pin(async move {
                        let request = *req
                            .downcast::<tonic::Request<super::HelloRequest>>()
                            .unwrap();
                        let res = inner
                            .say_hello(request)
                            .await
                            .expect("rpc handler returns error");
                        Ok(Box::new(res) as BoxMessage)
                    })
                }
                _ => Box::pin(async move { Ok(Box::new(()) as BoxMessage) }),
            }
        }
    }
    impl<T: Greeter> Clone for GreeterServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self { inner }
        }
    }
}
