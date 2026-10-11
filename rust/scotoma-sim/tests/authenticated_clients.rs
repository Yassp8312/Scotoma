use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use scotoma_client::{
    ClientConfig, ClientError, ClientProgress, DerivedClientConfig, LiveCommand, LiveEvent,
    LiveExit, authenticate, authenticate_derived,
};
use scotoma_protocol::{
    JoinRequest, NumericPassword, PasswordKdfParams, PeerRole, ProtocolLimits, RelayEvent,
    RoomCode, derive_rendezvous_tag, derive_session_secret,
};
use scotoma_relay::{RelayConfig, serve, serve_with_admin};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Barrier, mpsc, watch};
use tokio::task::{JoinHandle, JoinSet};
use zeroize::Zeroizing;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

async fn start_cuttable_proxy(
    target: SocketAddr,
) -> io::Result<(SocketAddr, watch::Sender<bool>, JoinHandle<io::Result<()>>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (cut, mut cut_rx) = watch::channel(false);
    let task = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (mut client, _) = accepted?;
                    let mut relay = TcpStream::connect(target).await?;
                    let mut connection_cut = cut_rx.clone();
                    connections.spawn(async move {
                        tokio::select! {
                            result = tokio::io::copy_bidirectional(&mut client, &mut relay) => {
                                result.map(|_| ())
                            }
                            changed = connection_cut.changed() => {
                                changed.map_err(io::Error::other)?;
                                Ok(())
                            }
                        }
                    });
                }
                changed = cut_rx.changed() => {
                    changed.map_err(io::Error::other)?;
                    break;
                }
            }
        }
        while let Some(result) = connections.join_next().await {
            result.map_err(io::Error::other)??;
        }
        Ok(())
    });
    Ok((address, cut, task))
}

#[tokio::test]
async fn two_clients_exchange_one_hundred_messages_then_burn() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let relay_config = RelayConfig {
        listen_addr: address,
        handshake_timeout: Duration::from_secs(2),
        protocol_limits: ProtocolLimits {
            max_pending_frames_per_room: 1,
            ..ProtocolLimits::default()
        },
        max_connection_frames_per_second: 5_000,
        connection_frame_burst: 5_000,
        max_room_frames_per_second: 10_000,
        room_frame_burst: 10_000,
        event_channel_capacity: 0,
        ..RelayConfig::default()
    };
    let server = tokio::spawn(async move { serve(listener, relay_config).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("314159")?;

    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("271828")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("271828")?,
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

    for counter in 0_u64..100 {
        let message = format!("authenticated message {counter:03}");
        let sending = first.send_text_and_wait_ack(&message);
        let receiving = second.receive_text_and_ack();
        let (acknowledged, received) = tokio::join!(sending, receiving);

        assert_eq!(acknowledged?, counter);
        assert_eq!(received?, message);
    }

    let burning = first.burn();
    let receiving_burn = second.wait_for_burn();
    let (burned, peer_burned) = tokio::join!(burning, receiving_burn);
    burned?;
    peer_burned?;

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn live_loops_exchange_bidirectionally_ack_and_burn() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("13")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("1370")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("1370")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (first_commands, first_receiver) = mpsc::channel(2);
    let (second_commands, second_receiver) = mpsc::channel(2);
    let (first_events, mut first_events_rx) = mpsc::unbounded_channel();
    let (second_events, mut second_events_rx) = mpsc::unbounded_channel();
    let first_task = tokio::spawn(first.run_live(
        first_receiver,
        move |event| {
            let _ = first_events.send(event);
        },
        || false,
    ));
    let second_task = tokio::spawn(second.run_live(
        second_receiver,
        move |event| {
            let _ = second_events.send(event);
        },
        || false,
    ));

    first_commands
        .send(LiveCommand::SendText(Zeroizing::new(
            "mensaje uno".as_bytes().to_vec(),
        )))
        .await?;
    assert!(matches!(
        second_events_rx.recv().await,
        Some(LiveEvent::ReceivedText(text)) if text.as_str() == "mensaje uno"
    ));
    assert!(matches!(
        first_events_rx.recv().await,
        Some(LiveEvent::Delivered(0))
    ));

    second_commands
        .send(LiveCommand::SendText(Zeroizing::new(
            "respuesta".as_bytes().to_vec(),
        )))
        .await?;
    assert!(matches!(
        first_events_rx.recv().await,
        Some(LiveEvent::ReceivedText(text)) if text.as_str() == "respuesta"
    ));
    assert!(matches!(
        second_events_rx.recv().await,
        Some(LiveEvent::Delivered(1))
    ));

    first_commands.send(LiveCommand::Burn).await?;
    assert_eq!(first_task.await??, LiveExit::LocalBurn);
    assert!(matches!(
        second_events_rx.recv().await,
        Some(LiveEvent::PeerBurned)
    ));
    assert_eq!(second_task.await??, LiveExit::PeerBurn);

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn missing_ack_is_terminal_and_never_retried() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("99")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("101")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("101")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (commands, receiver) = mpsc::channel(1);
    let task = tokio::spawn(first.run_live_with_ack_timeout(
        receiver,
        |_| {},
        || false,
        Duration::from_millis(50),
    ));
    commands
        .send(LiveCommand::SendText(Zeroizing::new(b"no retry".to_vec())))
        .await?;

    assert_eq!(task.await?, Err(ClientError::DeliveryTimeout));
    assert!(commands.send(LiveCommand::Burn).await.is_err());
    second.close().await?;
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn delayed_ack_before_deadline_is_still_delivered() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("300")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("400")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("400")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (first_commands, first_receiver) = mpsc::channel(2);
    let (first_events, mut first_events_rx) = mpsc::unbounded_channel();
    let first_task = tokio::spawn(first.run_live_with_ack_timeout(
        first_receiver,
        move |event| {
            let _ = first_events.send(event);
        },
        || false,
        Duration::from_millis(500),
    ));
    first_commands
        .send(LiveCommand::SendText(Zeroizing::new(
            b"delayed but valid".to_vec(),
        )))
        .await?;

    tokio::time::sleep(Duration::from_millis(75)).await;
    let (_second_commands, second_receiver) = mpsc::channel(1);
    let (second_events, mut second_events_rx) = mpsc::unbounded_channel();
    let second_task = tokio::spawn(second.run_live(
        second_receiver,
        move |event| {
            let _ = second_events.send(event);
        },
        || false,
    ));

    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), second_events_rx.recv()).await?,
        Some(LiveEvent::ReceivedText(text)) if text.as_str() == "delayed but valid"
    ));
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), first_events_rx.recv()).await?,
        Some(LiveEvent::Delivered(0))
    ));

    first_commands.send(LiveCommand::Burn).await?;
    assert_eq!(first_task.await??, LiveExit::LocalBurn);
    assert!(matches!(
        second_events_rx.recv().await,
        Some(LiveEvent::PeerBurned)
    ));
    assert_eq!(second_task.await??, LiveExit::PeerBurn);
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn network_cut_while_waiting_for_peer_is_terminal() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let (proxy_address, cut, proxy) = start_cuttable_proxy(address).await?;
    let room = RoomCode::parse("500")?;
    let password = NumericPassword::parse("600")?;
    let secret = derive_session_secret(&room, &password, PasswordKdfParams::default())?;
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let attempt = tokio::spawn(authenticate_derived(
        DerivedClientConfig {
            relay_url: format!("ws://{proxy_address}/v2/connect"),
            room,
            secret,
            operation_timeout: Duration::from_secs(2),
        },
        move |progress| {
            let _ = progress_tx.send(progress);
        },
    ));

    loop {
        let progress = tokio::time::timeout(Duration::from_secs(2), progress_rx.recv()).await?;
        if progress == Some(ClientProgress::WaitingForPeer) {
            break;
        }
    }
    cut.send(true)?;
    tokio::time::timeout(Duration::from_secs(1), proxy).await???;
    let result = tokio::time::timeout(Duration::from_secs(1), attempt)
        .await?
        .expect("authentication task must not panic");
    assert!(matches!(
        result,
        Err(ClientError::Connection | ClientError::RelayRejected)
    ));
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn peer_cut_during_noise_handshake_is_terminal_and_requires_a_fresh_session() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("610")?;
    let password = NumericPassword::parse("620")?;
    let secret = derive_session_secret(&room, &password, PasswordKdfParams::default())?;
    let tag = derive_rendezvous_tag(&room, &secret)?;
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
    let attempt = tokio::spawn(authenticate_derived(
        DerivedClientConfig {
            relay_url: relay_url.clone(),
            room: room.clone(),
            secret,
            operation_timeout: Duration::from_secs(2),
        },
        move |progress| {
            let _ = progress_tx.send(progress);
        },
    ));

    loop {
        let progress = tokio::time::timeout(Duration::from_secs(2), progress_rx.recv()).await?;
        if progress == Some(ClientProgress::WaitingForPeer) {
            break;
        }
    }

    let (mut peer, _) = tokio_tungstenite::connect_async(&relay_url).await?;
    peer.send(tokio_tungstenite::tungstenite::Message::Binary(
        JoinRequest { tag }.encode().into(),
    ))
    .await?;
    let ready = tokio::time::timeout(Duration::from_secs(2), peer.next())
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "peer was closed"))??;
    let ready = match ready {
        tokio_tungstenite::tungstenite::Message::Binary(packet) => packet,
        other => {
            return Err(io::Error::other(format!("expected peer-ready, got {other:?}")).into());
        }
    };
    assert_eq!(
        RelayEvent::decode(&ready)?,
        RelayEvent::PeerReady(PeerRole::Responder)
    );

    // Receiving the initiator's first Noise frame proves the cut happens after
    // pairing and during authentication, rather than during WebSocket setup.
    let first_handshake = tokio::time::timeout(Duration::from_secs(2), peer.next())
        .await?
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "handshake peer was closed")
        })??;
    assert!(matches!(
        first_handshake,
        tokio_tungstenite::tungstenite::Message::Binary(_)
    ));
    peer.close(None).await?;

    let result = tokio::time::timeout(Duration::from_secs(1), attempt)
        .await?
        .expect("authentication task must not panic");
    assert!(matches!(
        result,
        Err(ClientError::Connection | ClientError::RelayRejected)
    ));
    while let Ok(progress) = progress_rx.try_recv() {
        assert_ne!(progress, ClientProgress::Connected);
    }

    // Recovery is an explicit new session; the failed handshake cannot be
    // resumed and must not leave the room occupied.
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("620")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("620")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    first.close().await?;
    let _ = second.close().await;

    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn network_cut_during_live_chat_is_terminal() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let admin_listener = TcpListener::bind("127.0.0.1:0").await?;
    let admin_address = admin_listener.local_addr()?;
    let server = tokio::spawn(async move {
        serve_with_admin(listener, admin_listener, RelayConfig::default()).await
    });
    let (proxy_address, cut, proxy) = start_cuttable_proxy(address).await?;
    let relay_url = format!("ws://{proxy_address}/v2/connect");
    let room = RoomCode::parse("700")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("800")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("800")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (commands, receiver) = mpsc::channel(1);
    let task = tokio::spawn(first.run_live(receiver, |_| {}, || false));

    commands
        .send(LiveCommand::SendText(Zeroizing::new(
            b"pending during cut".to_vec(),
        )))
        .await?;
    wait_for_metric(admin_address, "scotoma_relay_pending_frames", 1).await?;
    cut.send(true)?;
    tokio::time::timeout(Duration::from_secs(1), proxy).await???;
    let result = tokio::time::timeout(Duration::from_secs(1), task)
        .await?
        .expect("live task must not panic");
    assert!(matches!(
        result,
        Err(ClientError::Connection | ClientError::RelayRejected)
    ));
    assert!(commands.send(LiveCommand::Burn).await.is_err());
    drop(second);
    wait_for_metric(admin_address, "scotoma_relay_pending_frames", 0).await?;
    wait_for_metric(admin_address, "scotoma_relay_active_rooms", 0).await?;
    server.abort();
    let _ = server.await;
    Ok(())
}

async fn get_metrics(address: SocketAddr) -> TestResult<String> {
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing metrics body"))?;
        Ok(body.to_owned())
    })
    .await??)
}

async fn wait_for_metric(address: SocketAddr, name: &str, expected: u64) -> TestResult {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let metrics = get_metrics(address).await?;
            let value = metrics.lines().find_map(|line| {
                let (candidate, value) = line.split_once(' ')?;
                (candidate == name)
                    .then(|| value.parse::<u64>().ok())
                    .flatten()
            });
            if value == Some(expected) {
                return Ok::<(), Box<dyn Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{name} did not reach {expected}"),
        )
    })??;
    Ok(())
}

#[tokio::test]
async fn burn_and_network_cut_race_always_ends_the_session() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let (proxy_address, cut, proxy) = start_cuttable_proxy(address).await?;
    let relay_url = format!("ws://{proxy_address}/v2/connect");
    let room = RoomCode::parse("1100")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("1200")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("1200")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (commands, receiver) = mpsc::channel(1);
    let task = tokio::spawn(first.run_live(receiver, |_| {}, || false));
    let barrier = Arc::new(Barrier::new(3));
    let burn_barrier = Arc::clone(&barrier);
    let burn = tokio::spawn(async move {
        burn_barrier.wait().await;
        let _ = commands.send(LiveCommand::Burn).await;
    });
    let cut_barrier = Arc::clone(&barrier);
    let cut_network = tokio::spawn(async move {
        cut_barrier.wait().await;
        let _ = cut.send(true);
    });

    barrier.wait().await;
    burn.await?;
    cut_network.await?;
    tokio::time::timeout(Duration::from_secs(1), proxy).await???;
    let result = tokio::time::timeout(Duration::from_secs(1), task)
        .await?
        .expect("live task must not panic");
    assert!(matches!(
        result,
        Ok(LiveExit::LocalBurn) | Err(ClientError::Connection | ClientError::RelayRejected)
    ));
    drop(second);
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn local_burn_preempts_a_pending_ack_deadline() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("900")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("1000")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("1000")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (commands, receiver) = mpsc::channel(2);
    let task = tokio::spawn(first.run_live_with_ack_timeout(
        receiver,
        |_| {},
        || false,
        Duration::from_secs(5),
    ));
    commands
        .send(LiveCommand::SendText(Zeroizing::new(
            b"burn while pending".to_vec(),
        )))
        .await?;
    tokio::time::sleep(Duration::from_millis(20)).await;
    commands.send(LiveCommand::Burn).await?;

    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), task).await???,
        LiveExit::LocalBurn
    );
    let _ = second.close().await;
    server.abort();
    let _ = server.await;
    Ok(())
}

#[tokio::test]
async fn peer_disconnect_is_terminal_for_the_live_loop() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { serve(listener, RelayConfig::default()).await });
    let relay_url = format!("ws://{address}/v2/connect");
    let room = RoomCode::parse("100")?;
    let first = authenticate(ClientConfig {
        relay_url: relay_url.clone(),
        room: room.clone(),
        password: NumericPassword::parse("200")?,
        operation_timeout: Duration::from_secs(2),
    });
    let second = authenticate(ClientConfig {
        relay_url,
        room,
        password: NumericPassword::parse("200")?,
        operation_timeout: Duration::from_secs(2),
    });
    let (first, second) = tokio::try_join!(first, second)?;
    let (_commands, receiver) = mpsc::channel(1);
    let task = tokio::spawn(first.run_live(receiver, |_| {}, || false));

    second.close().await?;
    assert!(matches!(
        task.await?,
        Err(ClientError::Connection | ClientError::RelayRejected)
    ));
    server.abort();
    let _ = server.await;
    Ok(())
}
