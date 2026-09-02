use std::error::Error;
use std::io;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::{Frame, FrameKind, ProtocolLimits, RENDEZVOUS_TAG_BYTES, RendezvousTag};
use scotoma_relay::{JoinRequest, PeerRole, RelayConfig, RelayEvent, serve};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[tokio::test]
async fn two_clients_pair_forward_a_frame_and_close_ephemerally() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x5a; RENDEZVOUS_TAG_BYTES]),
    };

    let mut first = connect_and_join(&url, &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::Waiting(PeerRole::Initiator)
    );

    let mut second = connect_and_join(&url, &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::PeerReady(PeerRole::Initiator)
    );
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::PeerReady(PeerRole::Responder)
    );

    let handshake = Frame::new(
        FrameKind::Handshake,
        0,
        b"opaque handshake bytes".to_vec(),
        ProtocolLimits::default(),
    )?
    .encode()?;
    first
        .send(Message::Binary(handshake.clone().into()))
        .await?;
    assert_eq!(next_binary(&mut second).await?, handshake);

    let mut third = connect_and_join(&url, &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut third).await?)?,
        RelayEvent::Error
    );

    first.send(Message::Close(None)).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn pending_frames_and_idle_rooms_expire_through_the_server() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let protocol_limits = ProtocolLimits {
        max_pending_frames_per_room: 1,
        ..ProtocolLimits::default()
    };
    let config = RelayConfig {
        listen_addr: address,
        protocol_limits,
        handshake_timeout: Duration::from_secs(2),
        pending_frame_ttl: Duration::from_millis(30),
        room_idle_ttl: Duration::from_millis(200),
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x6b; RENDEZVOUS_TAG_BYTES]),
    };
    let mut first = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let mut second = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let _ = next_binary(&mut second).await?;

    let first_pending = test_frame(FrameKind::Ciphertext, 0)?;
    first
        .send(Message::Binary(first_pending.clone().into()))
        .await?;
    assert_eq!(next_binary(&mut second).await?, first_pending);

    sleep(Duration::from_millis(90)).await;

    let after_ttl = test_frame(FrameKind::Ciphertext, 1)?;
    first
        .send(Message::Binary(after_ttl.clone().into()))
        .await?;
    assert_eq!(next_binary(&mut second).await?, after_ttl);

    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::RoomClosed
    );
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn global_upgrade_burst_is_rejected_before_join() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        max_websocket_upgrades_per_second: 1,
        websocket_upgrade_burst: 1,
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let (_first, _) = connect_async(&url).await?;

    let error = connect_async(&url)
        .await
        .expect_err("second immediate upgrade must exceed the global burst");

    assert_http_error(error, StatusCode::TOO_MANY_REQUESTS);
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn concurrent_connection_capacity_is_fail_closed() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        max_concurrent_connections: 1,
        max_websocket_upgrades_per_second: 10,
        websocket_upgrade_burst: 10,
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let (_first, _) = connect_async(&url).await?;

    let error = connect_async(&url)
        .await
        .expect_err("connection permit must be held for the live socket");

    assert_http_error(error, StatusCode::SERVICE_UNAVAILABLE);
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn per_connection_frame_burst_closes_the_room() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        max_connection_frames_per_second: 1,
        connection_frame_burst: 1,
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x72; RENDEZVOUS_TAG_BYTES]),
    };
    let mut first = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let mut second = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let _ = next_binary(&mut second).await?;
    let allowed = test_frame(FrameKind::Handshake, 0)?;
    let over_limit = test_frame(FrameKind::Handshake, 1)?;

    first.send(Message::Binary(allowed.clone().into())).await?;
    first.send(Message::Binary(over_limit.into())).await?;

    assert_eq!(next_binary(&mut second).await?, allowed);
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn missing_join_and_missing_peer_expire_independently() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_millis(30),
        pairing_timeout: Duration::from_millis(30),
        websocket_send_timeout: Duration::from_millis(100),
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let (mut no_join, _) = connect_async(&url).await?;

    assert_eq!(
        RelayEvent::decode(&next_binary(&mut no_join).await?)?,
        RelayEvent::Error
    );

    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x73; RENDEZVOUS_TAG_BYTES]),
    };
    let mut waiting = connect_and_join(&url, &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut waiting).await?)?,
        RelayEvent::Waiting(PeerRole::Initiator)
    );
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut waiting).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn paired_but_incomplete_handshake_expires_through_the_server() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(1),
        pairing_timeout: Duration::from_secs(1),
        session_handshake_timeout: Duration::from_millis(30),
        websocket_send_timeout: Duration::from_millis(100),
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x74; RENDEZVOUS_TAG_BYTES]),
    };
    let mut first = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let mut second = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;
    let _ = next_binary(&mut second).await?;

    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::RoomClosed
    );
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

async fn connect_and_join(url: &str, request: &JoinRequest) -> TestResult<ClientSocket> {
    let (mut socket, _) = connect_async(url).await?;
    socket
        .send(Message::Binary(request.encode().into()))
        .await?;
    Ok(socket)
}

async fn next_binary(socket: &mut ClientSocket) -> TestResult<Vec<u8>> {
    let received = timeout(Duration::from_secs(2), socket.next())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "WebSocket receive timed out"))?;
    let message = received
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "WebSocket closed"))??;
    match message {
        Message::Binary(packet) => Ok(packet.to_vec()),
        other => {
            Err(io::Error::other(format!("expected binary message, received {other:?}")).into())
        }
    }
}

fn test_frame(kind: FrameKind, counter: u64) -> TestResult<Vec<u8>> {
    Ok(Frame::new(kind, counter, vec![0x7c; 16], ProtocolLimits::default())?.encode()?)
}

fn assert_http_error(error: WebSocketError, expected: StatusCode) {
    let WebSocketError::Http(response) = error else {
        panic!("expected HTTP rejection, received {error}");
    };
    assert_eq!(response.status(), expected);
}
