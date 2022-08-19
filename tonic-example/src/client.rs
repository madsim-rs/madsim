// remove this after prost-build fix clippy issue
#![allow(clippy::derive_partial_eq_without_eq)]

use std::time::Duration;

use async_stream::stream;
use madsim::time::sleep;

use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://[::1]:50051").await?;

    println!("Testing unary ...");
    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("RESPONSE={:?}", response);
    println!();

    println!("Testing server streaming...");
    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.lots_of_replies(request).await?;
    let mut stream = response.into_inner();
    loop {
        match stream.message().await {
            Ok(Some(reply)) => println!("{:?}", reply),
            Ok(None) => break,
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }
    println!();

    println!("Testing client streaming...");
    let new_stream = || {
        stream! {
            for i in 0..3 {
                yield HelloRequest {
                    name: format!("Tonic{i}"),
                };
                sleep(Duration::from_secs(1)).await;
            }
        }
    };
    let response = client.lots_of_greetings(new_stream()).await?;
    println!("{:?}", response.into_inner());
    println!();

    println!("Testing bi-directional streaming...");
    let response = client.bidi_hello(new_stream()).await?;
    let mut stream = response.into_inner();
    while let Some(reply) = stream.message().await? {
        println!("{:?}", reply);
    }
    Ok(())
}
