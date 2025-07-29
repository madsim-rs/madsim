use std::pin::Pin;
use std::time::Duration;

use async_stream::try_stream;
use futures_core::Stream;
use madsim::time::sleep;
use tonic::{Request, Response, Status, Streaming};

use hello_world::another_greeter_server::AnotherGreeter;
use hello_world::greeter_server::Greeter;
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    // remove this after prost-build fix clippy issue
    #![allow(clippy::derive_partial_eq_without_eq)]

    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl AnotherGreeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {request:?}");
        let reply = HelloReply {
            message: format!("Hi {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }

    async fn delay(&self, request: Request<HelloRequest>) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {request:?}");
        sleep(Duration::from_secs(10)).await;
        let reply = HelloReply {
            message: format!("Hi {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {request:?}");
        let remote_addr = request.remote_addr().expect("no remote address");
        let name = request.into_inner().name;
        if name == "error" {
            return Err(Status::invalid_argument("error!"));
        }
        let reply = HelloReply {
            message: format!("Hello {}! ({})", name, remote_addr.ip()),
        };
        Ok(Response::new(reply))
    }

    type LotsOfRepliesStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn lots_of_replies(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::LotsOfRepliesStream>, Status> {
        println!("Got a request: {request:?}");
        let remote_addr = request.remote_addr().expect("no remote address");
        let stream = try_stream! {
            let name = request.into_inner().name;
            for i in 0..3 {
                yield HelloReply {
                    message: format!("{i}: Hello {name}! ({})", remote_addr.ip()),
                };
                sleep(Duration::from_secs(1)).await;
            }
            Err(Status::unknown("EOF"))?;
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn lots_of_greetings(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {request:?}");
        let remote_addr = request.remote_addr().expect("no remote address");
        let mut stream = request.into_inner();
        let mut s = String::new();
        while let Some(request) = stream.message().await? {
            println!("-> {request:?}");
            s += " ";
            s += &request.name;
        }
        let reply = HelloReply {
            message: format!("Hello{s}! ({})", remote_addr.ip()),
        };
        Ok(Response::new(reply))
    }

    type BidiHelloStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn bidi_hello(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<Self::BidiHelloStream>, Status> {
        println!("Got a request: {request:?}");
        let remote_addr = request.remote_addr().expect("no remote address");
        let stream = try_stream! {
            let mut stream = request.into_inner();
            while let Some(request) = stream.message().await? {
                println!("-> {request:?}");
                yield HelloReply {
                    message: format!("Hello {}! ({})", request.name, remote_addr.ip()),
                };
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }
}
