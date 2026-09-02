use std::io;
use std::sync::Arc;
use std::time::Instant as StandardInstant;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::Frame;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval_at, timeout};

use crate::hub::{Outbound, Registration, RelayHub};
use crate::rate_limit::RateLimiter;
use crate::{JoinRequest, PeerRole, RelayConfig, RelayEvent};

#[derive(Debug, Clone)]
struct AppState {
    config: RelayConfig,
    hub: RelayHub,
    connection_slots: Arc<Semaphore>,
    upgrade_limiter: Arc<Mutex<RateLimiter>>,
}

/// Builds the relay HTTP/WebSocket application.
pub fn app(config: RelayConfig) -> Router {
    let now = StandardInstant::now();
    let state = AppState {
        config,
        hub: RelayHub::new(config),
        connection_slots: Arc::new(Semaphore::new(config.max_concurrent_connections)),
        upgrade_limiter: Arc::new(Mutex::new(RateLimiter::new(
            config.max_websocket_upgrades_per_second,
            config.websocket_upgrade_burst,
            now,
        ))),
    };

    Router::new()
        .route("/health", get(health))
        .route("/v1/connect", any(connect))
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

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn connect(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    if !state
        .upgrade_limiter
        .lock()
        .await
        .allow(StandardInstant::now())
    {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(connection_permit) = Arc::clone(&state.connection_slots).try_acquire_owned() else {
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
            handle_socket(socket, state).await;
            drop(connection_permit);
        })
        .into_response()
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let Some((request, registration)) = accept_join(&mut socket, &state).await else {
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
                if !matches!(sent, Ok(Ok(()))) || closes_room {
                    break;
                }
            }
            incoming = socket_receiver.next() => {
                match incoming {
                    Some(Ok(message)) => {
                        if !frame_limiter.allow(StandardInstant::now()) {
                            break;
                        }
                        match message {
                            Message::Binary(encoded) => {
                                let Ok(frame) = Frame::decode(
                                    &encoded,
                                    state.config.protocol_limits,
                                ) else {
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
                                if !matches!(
                                    timeout(
                                        state.config.websocket_send_timeout,
                                        socket_sender.send(Message::Pong(payload)),
                                    ).await,
                                    Ok(Ok(()))
                                ) {
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
    let _ = timeout(state.config.websocket_send_timeout, socket_sender.close()).await;
}

async fn accept_join(
    socket: &mut WebSocket,
    state: &AppState,
) -> Option<(JoinRequest, Registration)> {
    let Ok(Some(Ok(Message::Binary(join_message)))) =
        timeout(state.config.handshake_timeout, socket.recv()).await
    else {
        reject(socket, state.config.websocket_send_timeout).await;
        return None;
    };

    let Ok(request) = JoinRequest::decode(&join_message) else {
        reject(socket, state.config.websocket_send_timeout).await;
        return None;
    };
    let Ok(registration) = state.hub.register(request.tag.clone()).await else {
        reject(socket, state.config.websocket_send_timeout).await;
        return None;
    };
    let initial_event = match registration.role {
        PeerRole::Initiator => RelayEvent::Waiting(PeerRole::Initiator),
        PeerRole::Responder => RelayEvent::PeerReady(PeerRole::Responder),
    };
    if !matches!(
        timeout(
            state.config.websocket_send_timeout,
            socket.send(Message::Binary(initial_event.encode().into()))
        )
        .await,
        Ok(Ok(()))
    ) {
        state.hub.unregister(&request.tag, registration.id).await;
        return None;
    }
    Some((request, registration))
}

async fn reject(socket: &mut WebSocket, send_timeout: std::time::Duration) {
    let _ = timeout(
        send_timeout,
        socket.send(Message::Binary(RelayEvent::Error.encode().into())),
    )
    .await;
    let _ = timeout(
        send_timeout,
        socket.send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: "protocol error".into(),
        }))),
    )
    .await;
}
