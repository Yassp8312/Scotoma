use std::error::Error;

use scotoma_relay::{DeploymentConfig, serve_with_admin};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = DeploymentConfig::from_environment()?;
    let public_listener = TcpListener::bind(config.relay.listen_addr).await?;
    let admin_listener = TcpListener::bind(config.admin_listen_addr).await?;

    println!(
        "scotoma-relay public listener: http://{} (WebSocket path /v1/connect)",
        config.relay.listen_addr
    );
    println!(
        "scotoma-relay private metrics listener: http://{}/metrics",
        config.admin_listen_addr
    );
    println!("public deployments must terminate TLS at a trusted edge and expose only WSS");

    serve_with_admin(public_listener, admin_listener, config.relay).await?;
    Ok(())
}
