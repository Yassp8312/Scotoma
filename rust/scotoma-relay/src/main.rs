use std::error::Error;

use scotoma_relay::{RelayConfig, serve};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = RelayConfig::default();
    let listener = TcpListener::bind(config.listen_addr).await?;

    println!(
        "scotoma-relay listening on ws://{}/v1/connect (loopback development only)",
        config.listen_addr
    );
    println!("public deployments must terminate TLS and expose WSS");

    serve(listener, config).await?;
    Ok(())
}
