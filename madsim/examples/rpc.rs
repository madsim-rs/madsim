use madsim::{net::Endpoint, Request};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Request)]
#[rtype("String")]
struct Echo(String);

#[derive(Clone)]
struct Server;

#[madsim::service]
impl Server {
    #[rpc]
    fn echo(&self, req: Echo) -> String {
        format!("echo: {}", req.0)
    }
}

#[tokio::main]
async fn main() {
    if let Some(addr) = std::env::args().nth(1) {
        // client
        let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
        let addr = addr.parse().unwrap();
        let reply = ep.call(addr, Echo("hello".into())).await.unwrap();
        println!("reply: {:?}", reply);
    } else {
        // server
        let ep = Endpoint::bind("127.0.0.1:0").await.unwrap();
        println!("listening on {}", ep.local_addr().unwrap());
        Server.serve_on(ep).await.unwrap();
    }
}
