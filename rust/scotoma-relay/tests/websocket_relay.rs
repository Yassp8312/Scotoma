use std::error::Error;
use std::io;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::{
    FRAME_HEADER_LEN, Frame, FrameKind, PROTOCOL_VERSION, ProtocolLimits, RENDEZVOUS_TAG_BYTES,
    RendezvousTag,
};
use scotoma_relay::{JoinRequest, PeerRole, RelayConfig, RelayEvent, serve, serve_with_admin};
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

#[tokio::test]
async fn metrics_endpoint_exposes_only_aggregate_process_state() -> TestResult {
    let public_listener = TcpListener::bind("127.0.0.1:0").await?;
    let public_address = public_listener.local_addr()?;
    let admin_listener = TcpListener::bind("127.0.0.1:0").await?;
    let admin_address = admin_listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: public_address,
        handshake_timeout: Duration::from_secs(1),
        pairing_timeout: Duration::from_secs(1),
        ..RelayConfig::default()
    };
    let server =
        tokio::spawn(
            async move { serve_with_admin(public_listener, admin_listener, config).await },
        );
    let url = format!("ws://{public_address}/v1/connect");
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x75; RENDEZVOUS_TAG_BYTES]),
    };
    let mut first = connect_and_join(&url, &request).await?;
    let _ = next_binary(&mut first).await?;

    assert!(
        get_http(public_address, "/metrics")
            .await?
            .starts_with("HTTP/1.1 404")
    );
    assert!(
        get_http(admin_address, "/health")
            .await?
            .starts_with("HTTP/1.1 404")
    );
    let active = get_metrics(admin_address).await?;

    assert!(active.contains("scotoma_relay_upgrades_accepted_total 1\n"));
    assert!(active.contains("scotoma_relay_active_connections 1\n"));
    assert!(active.contains("scotoma_relay_active_rooms 1\n"));
    assert!(active.contains("scotoma_relay_pending_frames 0\n"));
    assert!(!active.contains("rendezvous"));
    assert!(!active.contains("payload"));
    assert!(!active.contains("75757575"));

    first.send(Message::Close(None)).await?;
    sleep(Duration::from_millis(30)).await;
    let closed = get_metrics(admin_address).await?;
    assert!(closed.contains("scotoma_relay_active_connections 0\n"));
    assert!(closed.contains("scotoma_relay_active_rooms 0\n"));
    assert!(closed.contains("scotoma_relay_rooms_closed_total 1\n"));

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn malformed_protocol_frames_close_and_release_the_room() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");

    for (tag_byte, hostile_packet) in [0x80, 0x81, 0x82, 0x83, 0x84, 0x85]
        .into_iter()
        .zip(hostile_frame_corpus())
    {
        let request = JoinRequest {
            tag: RendezvousTag::from_bytes([tag_byte; RENDEZVOUS_TAG_BYTES]),
        };
        let (mut attacker, mut peer) = paired_clients(&url, &request).await?;

        attacker
            .send(Message::Binary(hostile_packet.into()))
            .await?;
        assert_eq!(
            RelayEvent::decode(&next_binary(&mut peer).await?)?,
            RelayEvent::RoomClosed
        );

        let mut replacement = connect_and_join(&url, &request).await?;
        assert_eq!(
            RelayEvent::decode(&next_binary(&mut replacement).await?)?,
            RelayEvent::Waiting(PeerRole::Initiator)
        );
        replacement.send(Message::Close(None)).await?;
    }

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn text_and_oversized_messages_fail_closed() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let maximum = config.max_websocket_message_bytes();
    let server = tokio::spawn(async move { serve(listener, config).await });
    let url = format!("ws://{address}/v1/connect");

    let text_request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x90; RENDEZVOUS_TAG_BYTES]),
    };
    let (mut text_sender, mut text_peer) = paired_clients(&url, &text_request).await?;
    text_sender.send(Message::Text("hostile".into())).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut text_peer).await?)?,
        RelayEvent::RoomClosed
    );

    let oversized_request = JoinRequest {
        tag: RendezvousTag::from_bytes([0x91; RENDEZVOUS_TAG_BYTES]),
    };
    let (mut oversized_sender, mut oversized_peer) =
        paired_clients(&url, &oversized_request).await?;
    oversized_sender
        .send(Message::Binary(vec![0_u8; maximum + 1].into()))
        .await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut oversized_peer).await?)?,
        RelayEvent::RoomClosed
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn fresh_relay_process_has_no_inherited_room_state() -> TestResult {
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes([0xa0; RENDEZVOUS_TAG_BYTES]),
    };
    let first_listener = TcpListener::bind("127.0.0.1:0").await?;
    let first_address = first_listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: first_address,
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let first_server = tokio::spawn(async move { serve(first_listener, config).await });
    let mut first = connect_and_join(&format!("ws://{first_address}/v1/connect"), &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::Waiting(PeerRole::Initiator)
    );

    first_server.abort();
    let _ = first_server.await;
    drop(first);

    let second_listener = TcpListener::bind("127.0.0.1:0").await?;
    let second_address = second_listener.local_addr()?;
    let second_config = RelayConfig {
        listen_addr: second_address,
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let second_server = tokio::spawn(async move { serve(second_listener, second_config).await });
    let mut after_restart =
        connect_and_join(&format!("ws://{second_address}/v1/connect"), &request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut after_restart).await?)?,
        RelayEvent::Waiting(PeerRole::Initiator)
    );

    second_server.abort();
    let _ = second_server.await;
    Ok(())
}

async fn connect_and_join(url: &str, request: &JoinRequest) -> TestResult<ClientSocket> {
    let (mut socket, _) = connect_async(url).await?;
    socket
        .send(Message::Binary(request.encode().into()))
        .await?;
    Ok(socket)
}

async fn paired_clients(
    url: &str,
    request: &JoinRequest,
) -> TestResult<(ClientSocket, ClientSocket)> {
    let mut first = connect_and_join(url, request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::Waiting(PeerRole::Initiator)
    );
    let mut second = connect_and_join(url, request).await?;
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut first).await?)?,
        RelayEvent::PeerReady(PeerRole::Initiator)
    );
    assert_eq!(
        RelayEvent::decode(&next_binary(&mut second).await?)?,
        RelayEvent::PeerReady(PeerRole::Responder)
    );
    Ok((first, second))
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

async fn get_metrics(address: std::net::SocketAddr) -> TestResult<String> {
    let response = get_http(address, "/metrics").await?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP body"))?;
    Ok(body.to_owned())
}

async fn get_http(address: std::net::SocketAddr, path: &'static str) -> TestResult<String> {
    Ok(tokio::task::spawn_blocking(move || -> io::Result<String> {
        use std::io::{Read as _, Write as _};

        let mut stream = std::net::TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    })
    .await??)
}

fn test_frame(kind: FrameKind, counter: u64) -> TestResult<Vec<u8>> {
    Ok(Frame::new(kind, counter, vec![0x7c; 16], ProtocolLimits::default())?.encode()?)
}

fn hostile_frame_corpus() -> Vec<Vec<u8>> {
    let mut wrong_version = vec![0_u8; FRAME_HEADER_LEN];
    wrong_version[0] = PROTOCOL_VERSION + 1;

    let mut wrong_kind = vec![0_u8; FRAME_HEADER_LEN];
    wrong_kind[0] = PROTOCOL_VERSION;
    wrong_kind[1] = 0xff;

    let mut missing_payload = vec![0_u8; FRAME_HEADER_LEN];
    missing_payload[0] = PROTOCOL_VERSION;
    missing_payload[1] = FrameKind::Ciphertext as u8;
    missing_payload[FRAME_HEADER_LEN - 1] = 1;

    let mut trailing_payload = vec![0_u8; FRAME_HEADER_LEN + 1];
    trailing_payload[0] = PROTOCOL_VERSION;
    trailing_payload[1] = FrameKind::Ciphertext as u8;

    vec![
        Vec::new(),
        vec![0_u8; FRAME_HEADER_LEN - 1],
        wrong_version,
        wrong_kind,
        missing_payload,
        trailing_payload,
    ]
}

fn assert_http_error(error: WebSocketError, expected: StatusCode) {
    let WebSocketError::Http(response) = error else {
        panic!("expected HTTP rejection, received {error}");
    };
    assert_eq!(response.status(), expected);
}
