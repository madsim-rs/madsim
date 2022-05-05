use futures_core::Stream;
use std::pin::Pin;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    // tonic::include_proto!("helloworld");
    include!("generated.rs");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }

    type LotsOfRepliesStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn lots_of_replies(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::LotsOfRepliesStream>, Status> {
        todo!()
    }

    async fn lots_of_greetings(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<HelloReply>, Status> {
        todo!()
    }

    type BidiHelloStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn bidi_hello(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<Self::BidiHelloStream>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
