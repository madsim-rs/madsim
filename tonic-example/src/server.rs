use std::pin::Pin;
use std::time::Duration;

use async_stream::try_stream;
use futures_core::Stream;
use madsim::time::sleep;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    #[cfg(not(feature = "sim"))]
    tonic::include_proto!("helloworld");
    #[cfg(feature = "sim")]
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
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }

    type LotsOfRepliesStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn lots_of_replies(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<Self::LotsOfRepliesStream>, Status> {
        println!("Got a request: {:?}", request);
        let stream = try_stream! {
            let name = request.into_inner().name;
            for i in 0..3 {
                yield HelloReply {
                    message: format!("{i}: Hello {name}!"),
                };
                sleep(Duration::from_secs(1)).await;
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn lots_of_greetings(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);
        let mut stream = request.into_inner();
        let mut s = String::new();
        while let Some(request) = stream.message().await? {
            println!("-> {:?}", request);
            s += " ";
            s += &request.name;
        }
        let reply = HelloReply {
            message: format!("Hello{s}!"),
        };
        Ok(Response::new(reply))
    }

    type BidiHelloStream = Pin<Box<dyn Stream<Item = Result<HelloReply, Status>> + Send>>;

    async fn bidi_hello(
        &self,
        request: Request<Streaming<HelloRequest>>,
    ) -> Result<Response<Self::BidiHelloStream>, Status> {
        println!("Got a request: {:?}", request);
        let stream = try_stream! {
            let mut stream = request.into_inner();
            while let Some(request) = stream.message().await? {
                println!("-> {:?}", request);
                yield HelloReply {
                    message: format!("Hello {}!", request.name),
                };
            }
        };
        Ok(Response::new(Box::pin(stream)))
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

#[cfg(test)]
mod tests {
    use super::hello_world::greeter_client::GreeterClient;
    use madsim::{runtime::Handle, time::sleep};
    use std::net::SocketAddr;

    use super::*;

    #[madsim::test]
    async fn test() {
        let handle = Handle::current();
        let addr1 = "10.0.0.1:50051".parse::<SocketAddr>().unwrap();
        let addr2 = "10.0.0.2:0".parse::<SocketAddr>().unwrap();
        let node1 = handle.create_node().name("server").ip(addr1.ip()).build();
        let node2 = handle.create_node().name("client").ip(addr2.ip()).build();

        node1
            .spawn(async move {
                let greeter = MyGreeter::default();
                Server::builder()
                    .add_service(GreeterServer::new(greeter))
                    .serve(addr1)
                    .await
                    .unwrap();
            })
            .detach();

        node2
            .spawn(async move {
                sleep(Duration::from_secs(1)).await;
                let mut client = GreeterClient::connect("http://10.0.0.1:50051")
                    .await
                    .unwrap();
                let request = tonic::Request::new(HelloRequest {
                    name: "Tonic".into(),
                });
                let response = client.say_hello(request).await.unwrap();
                assert_eq!(response.into_inner().message, "Hello Tonic!");
            })
            .await
    }
}
