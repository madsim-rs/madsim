use tonic::transport::Server;
use tonic_example::MyGreeter;
use tonic_example::hello_world::{
    another_greeter_server::AnotherGreeterServer, greeter_server::GreeterServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    Server::builder()
        .add_service(GreeterServer::new(MyGreeter::default()))
        .add_service(AnotherGreeterServer::new(MyGreeter::default()))
        .serve(addr)
        .await?;

    Ok(())
}
