use std::error::Error;

use scotoma_relay::{DeploymentConfig, serve_with_admin_shutdown};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = DeploymentConfig::from_environment()?;
    let public_listener = TcpListener::bind(config.relay.listen_addr).await?;
    let admin_listener = TcpListener::bind(config.admin_listen_addr).await?;

    println!(
        "scotoma-relay public listener: http://{} (WebSocket path /v2/connect)",
        config.relay.listen_addr
    );
    println!(
        "scotoma-relay private metrics listener: http://{}/metrics",
        config.admin_listen_addr
    );
    println!("public deployments must terminate TLS at a trusted edge and expose only WSS");

    serve_with_admin_shutdown(
        public_listener,
        admin_listener,
        config.relay,
        shutdown_signal(),
    )
    .await?;
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
