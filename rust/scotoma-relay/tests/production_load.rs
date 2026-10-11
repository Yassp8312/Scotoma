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
const CHURN_ROUNDS: usize = 6;
const CHURN_ROOMS_PER_ROUND: usize = 24;
const CHURN_PAYLOAD_BYTES: usize = 16 * 1_024;
const DEFAULT_SOAK_ROUNDS: usize = 50;
const DEFAULT_SOAK_ROOMS_PER_ROUND: usize = 16;
const MAX_SOAK_ROUNDS: usize = 10_000;
const MAX_SOAK_ROOMS_PER_ROUND: usize = 128;

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

    let metrics = wait_for_zero_resource_gauges(admin_address).await?;
    assert_eq!(
        metric_value(&metrics, "scotoma_relay_rooms_created_total"),
        Some(40)
    );
    assert_eq!(
        metric_value(&metrics, "scotoma_relay_rooms_closed_total"),
        Some(40)
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn repeated_pending_disconnect_churn_releases_every_resource() -> TestResult {
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

    timeout(Duration::from_secs(30), async {
        for round in 0..CHURN_ROUNDS {
            let first_index = round * CHURN_ROOMS_PER_ROUND;
            stream::iter(
                (0..CHURN_ROOMS_PER_ROUND)
                    .map(|offset| exercise_pending_disconnect(url.clone(), first_index + offset)),
            )
            .buffer_unordered(CHURN_ROOMS_PER_ROUND)
            .for_each(|result| async move {
                result.expect("each pending disconnect path must succeed");
            })
            .await;

            wait_for_zero_resource_gauges(admin_address)
                .await
                .expect("each churn round must release every resource gauge");
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "churn profile timed out"))?;

    let metrics = get_metrics(admin_address).await?;
    let expected_rooms = u64::try_from(CHURN_ROUNDS * CHURN_ROOMS_PER_ROUND)?;
    assert_eq!(
        metric_value(&metrics, "scotoma_relay_rooms_created_total"),
        Some(expected_rooms)
    );
    assert_eq!(
        metric_value(&metrics, "scotoma_relay_rooms_closed_total"),
        Some(expected_rooms)
    );

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "configurable prolonged soak; run through rust/scotoma-relay/run_soak.ps1"]
async fn prolonged_mixed_cleanup_soak_returns_every_gauge_to_zero() -> TestResult {
    let rounds = bounded_env_usize("SCOTOMA_SOAK_ROUNDS", DEFAULT_SOAK_ROUNDS, MAX_SOAK_ROUNDS)?;
    let rooms_per_round = bounded_env_usize(
        "SCOTOMA_SOAK_ROOMS_PER_ROUND",
        DEFAULT_SOAK_ROOMS_PER_ROUND,
        MAX_SOAK_ROOMS_PER_ROUND,
    )?;
    let public_listener = TcpListener::bind("127.0.0.1:0").await?;
    let public_address = public_listener.local_addr()?;
    let admin_listener = TcpListener::bind("127.0.0.1:0").await?;
    let admin_address = admin_listener.local_addr()?;
    let config = RelayConfig {
        listen_addr: public_address,
        max_rooms: rooms_per_round,
        handshake_timeout: Duration::from_secs(5),
        pairing_timeout: Duration::from_secs(5),
        // Admission pressure and its puzzle path have dedicated tests. This
        // campaign isolates lifecycle stability across many sequential rooms.
        max_unchallenged_upgrades_per_second: u32::MAX,
        unchallenged_upgrade_burst: u32::MAX,
        max_websocket_upgrades_per_second: u32::MAX,
        websocket_upgrade_burst: u32::MAX,
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let server =
        tokio::spawn(
            async move { serve_with_admin(public_listener, admin_listener, config).await },
        );
    let url = format!("ws://{public_address}/v2/connect");
    let total_rooms = rounds
        .checked_mul(rooms_per_round)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "soak room count overflow"))?;
    let timeout_seconds = u64::try_from(rounds)?.saturating_mul(3).saturating_add(30);

    timeout(Duration::from_secs(timeout_seconds), async {
        for round in 0..rounds {
            let first_index = round * rooms_per_round;
            stream::iter((0..rooms_per_round).map(|offset| {
                let room_url = url.clone();
                async move {
                    let index = first_index + offset;
                    if (round + offset) % 3 == 0 {
                        exercise_room(room_url, index).await
                    } else {
                        exercise_pending_disconnect(room_url, index).await
                    }
                }
            }))
            .buffer_unordered(rooms_per_round)
            .for_each(|result| async move {
                result.expect("each soak room path must succeed");
            })
            .await;

            let metrics = wait_for_zero_resource_gauges(admin_address)
                .await
                .expect("every soak round must release all resource gauges");
            let expected_closed = u64::try_from((round + 1) * rooms_per_round)
                .expect("bounded soak room count fits u64");
            assert_eq!(
                metric_value(&metrics, "scotoma_relay_rooms_created_total"),
                Some(expected_closed)
            );
            assert_eq!(
                metric_value(&metrics, "scotoma_relay_rooms_closed_total"),
                Some(expected_closed)
            );
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "mixed soak timed out"))?;

    println!("SCOTOMA_SOAK_OK rounds={rounds} rooms={total_rooms}");
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

async fn exercise_pending_disconnect(url: String, index: usize) -> TestResult {
    let mut tag_bytes = [0_u8; RENDEZVOUS_TAG_BYTES];
    tag_bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
    tag_bytes[8] = 0x5a;
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

    let ciphertext = Frame::new(
        FrameKind::Ciphertext,
        0,
        vec![0xa5; CHURN_PAYLOAD_BYTES],
        ProtocolLimits::default(),
    )?
    .encode()?;
    first
        .send(Message::Binary(ciphertext.clone().into()))
        .await?;
    assert_eq!(next_binary(&mut second).await?, ciphertext);

    // Deliberately omit ACK and BURN. Disconnect cleanup must destroy the room
    // and purge the pending ciphertext without leaving observable resources.
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

async fn wait_for_zero_resource_gauges(address: std::net::SocketAddr) -> TestResult<String> {
    timeout(Duration::from_secs(5), async {
        loop {
            let metrics = get_metrics(address).await?;
            let gauges_are_zero = [
                "scotoma_relay_active_rooms",
                "scotoma_relay_pending_frames",
                "scotoma_relay_pending_bytes",
                "scotoma_relay_active_connections",
            ]
            .into_iter()
            .all(|name| metric_value(&metrics, name) == Some(0));
            if gauges_are_zero {
                return Ok(metrics);
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            "resource gauges did not return to zero",
        )
    })?
}

fn metric_value(metrics: &str, name: &str) -> Option<u64> {
    metrics.lines().find_map(|line| {
        let (candidate, value) = line.split_once(' ')?;
        (candidate == name).then(|| value.parse().ok()).flatten()
    })
}

fn bounded_env_usize(name: &str, default: usize, maximum: usize) -> TestResult<usize> {
    let value = match std::env::var(name) {
        Ok(raw) => raw.parse::<usize>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must be a positive integer"),
            )
        })?,
        Err(std::env::VarError::NotPresent) => default,
        Err(error) => return Err(error.into()),
    };
    if value == 0 || value > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be between 1 and {maximum}"),
        )
        .into());
    }
    Ok(value)
}
