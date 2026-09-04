use std::error::Error;
use std::io;
use std::time::Duration;

use futures_util::stream;
use futures_util::{SinkExt as _, StreamExt as _};
use scotoma_protocol::{
    Frame, FrameKind, JoinRequest, PeerRole, ProtocolLimits, RENDEZVOUS_TAG_BYTES, RelayEvent,
    RendezvousTag,
};
use scotoma_relay::{RelayConfig, serve_with_admin};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const CONCURRENT_ROOMS: usize = 40;
const MESSAGES_PER_ROOM: u64 = 20;
const PAYLOAD_BYTES: usize = 1_024;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn production_profile_handles_bounded_concurrent_rooms_and_returns_to_zero() -> TestResult {
    let public_listener = TcpListener::bind("127.0.0.1:0").await?;
    let public_address = public_listener.local_addr()?;
    let admin_listener = TcpListener::bind("127.0.0.1:0").await?;
    let admin_address = admin_listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: public_address,
        handshake_timeout: Duration::from_secs(5),
        pairing_timeout: Duration::from_secs(5),
        ..RelayConfig::default()
    };
    let server =
        tokio::spawn(
            async move { serve_with_admin(public_listener, admin_listener, config).await },
        );
    let url = format!("ws://{public_address}/v2/connect");

    timeout(
        Duration::from_secs(20),
        stream::iter((0..CONCURRENT_ROOMS).map(|index| exercise_room(url.clone(), index)))
            .buffer_unordered(CONCURRENT_ROOMS)
            .for_each(|result| async move {
                result.expect("each bounded room load path must succeed");
            }),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "load profile timed out"))?;

    sleep(Duration::from_millis(100)).await;
    let metrics = get_metrics(admin_address).await?;
    assert!(metrics.contains("scotoma_relay_active_rooms 0\n"));
    assert!(metrics.contains("scotoma_relay_pending_frames 0\n"));
    assert!(metrics.contains("scotoma_relay_active_connections 0\n"));

    server.abort();
    let _ = server.await;
    Ok(())
}

async fn exercise_room(url: String, index: usize) -> TestResult {
    let mut tag_bytes = [0_u8; RENDEZVOUS_TAG_BYTES];
    tag_bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
    tag_bytes[8] = 0xa5;
    let request = JoinRequest {
        tag: RendezvousTag::from_bytes(tag_bytes),
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

    for counter in 0..MESSAGES_PER_ROOM {
        let ciphertext = Frame::new(
            FrameKind::Ciphertext,
            counter,
            vec![0x5a; PAYLOAD_BYTES],
            ProtocolLimits::default(),
        )?
        .encode()?;
        first
            .send(Message::Binary(ciphertext.clone().into()))
            .await?;
        assert_eq!(next_binary(&mut second).await?, ciphertext);

        let ack = Frame::new(
            FrameKind::Ack,
            counter,
            counter.to_be_bytes().to_vec(),
            ProtocolLimits::default(),
        )?
        .encode()?;
        second.send(Message::Binary(ack.clone().into())).await?;
        assert_eq!(next_binary(&mut first).await?, ack);
    }

    let burn = Frame::new(
        FrameKind::Burn,
        MESSAGES_PER_ROOM,
        Vec::new(),
        ProtocolLimits::default(),
    )?
    .encode()?;
    first.send(Message::Binary(burn.clone().into())).await?;
    assert_eq!(next_binary(&mut second).await?, burn);

    drop(first);
    drop(second);
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
    let received = timeout(Duration::from_secs(5), socket.next())
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
    Ok(tokio::task::spawn_blocking(move || -> io::Result<String> {
        use std::io::{Read as _, Write as _};

        let mut stream = std::net::TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let (_, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP body"))?;
        Ok(body.to_owned())
    })
    .await??)
}
