use std::future::Future;
use std::io;
use std::sync::Arc;
use std::time::Instant as StandardInstant;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::{AdmissionChallenge, Frame, PuzzleSolution, verify_admission_solution};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Semaphore, watch};
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval_at, timeout};

use crate::hub::{Outbound, Registration, RelayHub};
use crate::observability::{Observability, OperationalEvent};
use crate::rate_limit::RateLimiter;
use crate::{JoinRequest, PeerRole, RelayConfig, RelayEvent};

#[derive(Debug, Clone)]
struct AppState {
    config: RelayConfig,
    hub: RelayHub,
    connection_slots: Arc<Semaphore>,
    unchallenged_upgrade_limiter: Arc<Mutex<RateLimiter>>,
    upgrade_limiter: Arc<Mutex<RateLimiter>>,
    observability: Observability,
}

/// Builds the relay HTTP/WebSocket application.
pub fn app(config: RelayConfig) -> Router {
    public_router(app_state(config))
}

fn app_state(config: RelayConfig) -> AppState {
    let now = StandardInstant::now();
    let observability = Observability::with_stderr_sink(config.event_channel_capacity);
    AppState {
        config,
        hub: RelayHub::with_observability(config, observability.clone()),
        connection_slots: Arc::new(Semaphore::new(config.max_concurrent_connections)),
        unchallenged_upgrade_limiter: Arc::new(Mutex::new(RateLimiter::new(
            config.max_unchallenged_upgrades_per_second,
            config.unchallenged_upgrade_burst,
            now,
        ))),
        upgrade_limiter: Arc::new(Mutex::new(RateLimiter::new(
            config.max_websocket_upgrades_per_second,
            config.websocket_upgrade_burst,
            now,
        ))),
        observability,
    }
}

fn public_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v2/connect", any(connect))
        .with_state(state)
}

fn admin_router(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(metrics))
        .with_state(state)
}

/// Serves the relay on an already-bound listener.
///
/// # Errors
///
/// Returns an I/O error if the HTTP server fails.
pub async fn serve(listener: TcpListener, config: RelayConfig) -> io::Result<()> {
    axum::serve(listener, app(config)).await
}

/// Serves public relay traffic and private metrics on separate listeners.
///
/// # Errors
///
/// Returns an I/O error if either HTTP server fails.
pub async fn serve_with_admin(
    public_listener: TcpListener,
    admin_listener: TcpListener,
    config: RelayConfig,
) -> io::Result<()> {
    let state = app_state(config);
    let public_server = axum::serve(public_listener, public_router(state.clone()));
    let admin_server = axum::serve(admin_listener, admin_router(state));
    tokio::try_join!(public_server, admin_server)?;
    Ok(())
}

/// Serves both listeners and drains them after the supplied shutdown signal.
///
/// Existing `WebSocket` connections are given the container's grace period to finish. Any
/// remaining session is then destroyed with the process; no state is persisted
/// for a later deployment.
///
/// # Errors
///
/// Returns an I/O error if either HTTP server fails.
pub async fn serve_with_admin_shutdown<F>(
    public_listener: TcpListener,
    admin_listener: TcpListener,
    config: RelayConfig,
    shutdown: F,
) -> io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let state = app_state(config);
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let shutdown_task = tokio::spawn(async move {
        shutdown.await;
        let _ = shutdown_sender.send(true);
    });

    let public_server = axum::serve(public_listener, public_router(state.clone()))
        .with_graceful_shutdown(wait_for_shutdown(shutdown_receiver.clone()));
    let admin_server = axum::serve(admin_listener, admin_router(state))
        .with_graceful_shutdown(wait_for_shutdown(shutdown_receiver));
    let result = tokio::try_join!(public_server, admin_server).map(|_| ());
    shutdown_task.abort();
    result
}

async fn wait_for_shutdown(mut receiver: watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    let _ = receiver.changed().await;
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn metrics(State(state): State<AppState>) -> Response {
    (
        [("content-type", "text/plain; version=0.0.4")],
        state.observability.snapshot().render(),
    )
        .into_response()
}

async fn connect(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    if !state
        .upgrade_limiter
        .lock()
        .await
        .allow(StandardInstant::now())
    {
        state
            .observability
            .record(OperationalEvent::UpgradeRateLimited);
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let requires_puzzle = !state
        .unchallenged_upgrade_limiter
        .lock()
        .await
        .allow(StandardInstant::now());
    let Ok(connection_permit) = Arc::clone(&state.connection_slots).try_acquire_owned() else {
        state
            .observability
            .record(OperationalEvent::ConnectionCapacityRejected);
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let maximum = state
        .config
        .max_websocket_message_bytes()
        .max(RelayConfig::join_packet_bytes());

    upgrade
        .max_message_size(maximum)
        .max_frame_size(maximum)
        .on_upgrade(move |socket| async move {
            let observability = state.observability.clone();
            observability.connection_opened();
            handle_socket(socket, state, requires_puzzle).await;
            observability.connection_closed();
            drop(connection_permit);
        })
        .into_response()
}

async fn handle_socket(mut socket: WebSocket, state: AppState, requires_puzzle: bool) {
    let Some((request, registration)) = accept_join(&mut socket, &state, requires_puzzle).await
    else {
        return;
    };

    let peer_id = registration.id;
    let mut outbound = registration.receiver;
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let cleanup_interval = state.config.cleanup_interval();
    let mut cleanup = interval_at(TokioInstant::now() + cleanup_interval, cleanup_interval);
    cleanup.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut frame_limiter = RateLimiter::new(
        state.config.max_connection_frames_per_second,
        state.config.connection_frame_burst,
        StandardInstant::now(),
    );

    loop {
        tokio::select! {
            _ = cleanup.tick() => {
                let _ = state.hub.expire(&request.tag).await;
            }
            outgoing = outbound.recv() => {
                let Some(outgoing) = outgoing else {
                    break;
                };
                let closes_room = matches!(outgoing, Outbound::Event(RelayEvent::RoomClosed));
                let packet = match outgoing {
                    Outbound::Frame(frame) => frame,
                    Outbound::Event(event) => event.encode(),
                };
                let sent = timeout(
                    state.config.websocket_send_timeout,
                    socket_sender.send(Message::Binary(packet.into())),
                ).await;
                if sent.is_err() {
                    state.observability.record(OperationalEvent::SlowWriter);
                }
                if !matches!(sent, Ok(Ok(()))) || closes_room {
                    break;
                }
            }
            incoming = socket_receiver.next() => {
                match incoming {
                    Some(Ok(message)) => {
                        if !frame_limiter.allow(StandardInstant::now()) {
                            state
                                .observability
                                .record(OperationalEvent::ConnectionFrameRateLimited);
                            break;
                        }
                        match message {
                            Message::Binary(encoded) => {
                                let Ok(frame) = Frame::decode(
                                    &encoded,
                                    state.config.protocol_limits,
                                ) else {
                                    state
                                        .observability
                                        .record(OperationalEvent::ProtocolRejected);
                                    break;
                                };
                                let Ok(canonical) = frame.encode() else {
                                    break;
                                };
                                if state
                                    .hub
                                    .process_frame(&request.tag, peer_id, &frame, canonical)
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Message::Ping(payload) => {
                                let sent = timeout(
                                    state.config.websocket_send_timeout,
                                    socket_sender.send(Message::Pong(payload)),
                                ).await;
                                if sent.is_err() {
                                    state.observability.record(OperationalEvent::SlowWriter);
                                }
                                if !matches!(sent, Ok(Ok(()))) {
                                    break;
                                }
                            }
                            Message::Pong(_) => {}
                            Message::Close(_) | Message::Text(_) => break,
                        }
                    }
                    Some(Err(_)) | None => break,
                }
            }
        }
    }

    state.hub.unregister(&request.tag, peer_id).await;
    close_sender(&mut socket_sender, &state).await;
}

async fn accept_join(
    socket: &mut WebSocket,
    state: &AppState,
    requires_puzzle: bool,
) -> Option<(JoinRequest, Registration)> {
    let join_message = match timeout(state.config.handshake_timeout, socket.recv()).await {
        Ok(Some(Ok(Message::Binary(join_message)))) => join_message,
        Err(_) => {
            state.observability.record(OperationalEvent::JoinTimedOut);
            reject(socket, state).await;
            return None;
        }
        Ok(Some(Ok(_) | Err(_)) | None) => {
            state.observability.record(OperationalEvent::JoinRejected);
            reject(socket, state).await;
            return None;
        }
    };

    let Ok(request) = JoinRequest::decode(&join_message) else {
        state.observability.record(OperationalEvent::JoinRejected);
        reject(socket, state).await;
        return None;
    };
    if requires_puzzle && !complete_admission_puzzle(socket, state, &request).await {
        state.observability.record(OperationalEvent::JoinRejected);
        reject(socket, state).await;
        return None;
    }
    let Ok(registration) = state.hub.register(request.tag.clone()).await else {
        state.observability.record(OperationalEvent::JoinRejected);
        reject(socket, state).await;
        return None;
    };
    let initial_event = match registration.role {
        PeerRole::Initiator => RelayEvent::Waiting(PeerRole::Initiator),
        PeerRole::Responder => RelayEvent::PeerReady(PeerRole::Responder),
    };
    let sent = timeout(
        state.config.websocket_send_timeout,
        socket.send(Message::Binary(initial_event.encode().into())),
    )
    .await;
    if sent.is_err() {
        state.observability.record(OperationalEvent::SlowWriter);
    }
    if !matches!(sent, Ok(Ok(()))) {
        state.hub.unregister(&request.tag, registration.id).await;
        return None;
    }
    Some((request, registration))
}

async fn complete_admission_puzzle(
    socket: &mut WebSocket,
    state: &AppState,
    request: &JoinRequest,
) -> bool {
    let mut nonce = [0_u8; 16];
    if getrandom::fill(&mut nonce).is_err() {
        return false;
    }
    let challenge = AdmissionChallenge {
        nonce,
        difficulty_bits: state.config.admission_puzzle_difficulty_bits,
    };
    if !challenge.is_valid() {
        return false;
    }
    let sent = timeout(
        state.config.websocket_send_timeout,
        socket.send(Message::Binary(
            RelayEvent::PuzzleChallenge(challenge).encode().into(),
        )),
    )
    .await;
    if !matches!(sent, Ok(Ok(()))) {
        return false;
    }
    let solution = timeout(state.config.handshake_timeout, socket.recv()).await;
    let Ok(Some(Ok(Message::Binary(packet)))) = solution else {
        return false;
    };
    let Ok(solution) = PuzzleSolution::decode(&packet) else {
        return false;
    };
    verify_admission_solution(challenge, &request.tag, solution.value)
}

async fn close_sender(sender: &mut SplitSink<WebSocket, Message>, state: &AppState) {
    if timeout(state.config.websocket_send_timeout, sender.close())
        .await
        .is_err()
    {
        state.observability.record(OperationalEvent::SlowWriter);
    }
}

async fn reject(socket: &mut WebSocket, state: &AppState) {
    let event_sent = timeout(
        state.config.websocket_send_timeout,
        socket.send(Message::Binary(RelayEvent::Error.encode().into())),
    )
    .await;
    let close_sent = timeout(
        state.config.websocket_send_timeout,
        socket.send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: "protocol error".into(),
        }))),
    )
    .await;
    if event_sent.is_err() || close_sent.is_err() {
        state.observability.record(OperationalEvent::SlowWriter);
    }
}
