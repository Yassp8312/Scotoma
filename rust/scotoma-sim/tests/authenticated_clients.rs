use std::error::Error;
use std::time::Duration;

use scotoma_protocol::{InviteSecret, PeerRole, ProtocolLimits, RoomCode};
use scotoma_relay::{RelayConfig, serve};
use scotoma_sim::{ClientConfig, authenticate};
use tokio::net::TcpListener;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn two_clients_exchange_messages_then_burn_through_the_real_relay() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let relay_config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        protocol_limits: ProtocolLimits {
            max_pending_frames_per_room: 1,
            ..ProtocolLimits::default()
        },
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, relay_config).await });
    let relay_url = format!("ws://{address}/v1/connect");
    let room = RoomCode::new(314_159)?;
    let secret_bytes = [0x5a; 32];

    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room,
        secret: InviteSecret::from_bytes(secret_bytes),
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        secret: InviteSecret::from_bytes(secret_bytes),
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::join!(first, second);
    let mut first = first?;
    let mut second = second?;

    assert_eq!(first.role(), PeerRole::Initiator);
    assert_eq!(second.role(), PeerRole::Responder);
    assert_eq!(
        first.handshake_hash().expect("confirmed first hash"),
        second.handshake_hash().expect("confirmed second hash")
    );

    let sending = first.send_text_and_wait_ack("hello through the relay");
    let receiving = second.receive_text_and_ack();
    let (acknowledged, received) = tokio::join!(sending, receiving);

    assert_eq!(acknowledged?, 0);
    assert_eq!(received?, "hello through the relay");

    let sending = first.send_text_and_wait_ack("second message after ACK purge");
    let receiving = second.receive_text_and_ack();
    let (acknowledged, received) = tokio::join!(sending, receiving);

    assert_eq!(acknowledged?, 1);
    assert_eq!(received?, "second message after ACK purge");

    let burning = first.burn();
    let receiving_burn = second.wait_for_burn();
    let (burned, peer_burned) = tokio::join!(burning, receiving_burn);
    burned?;
    peer_burned?;

    server.abort();
    let _ = server.await;
    Ok(())
}
