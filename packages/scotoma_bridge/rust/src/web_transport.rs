//! Browser WebSocket transport for the production opaque session.
//!
//! The browser owns only framed ciphertext and handshake packets. Room,
//! password, derived secret, Noise state and plaintext stay inside Rust/Wasm.

use std::time::Duration;

use futures_channel::mpsc::{self, UnboundedReceiver};
use futures_util::{FutureExt, StreamExt, pin_mut, select};
use js_sys::{ArrayBuffer, Promise, Uint8Array};
use scotoma_protocol::{
    FRAME_HEADER_LEN, Frame, FrameKind, HandshakeRole, HandshakeSession, JoinRequest, PeerRole,
    ProtocolLimits, PuzzleSolution, RelayEvent, RendezvousTag, RoomCode, SessionSecret,
    derive_rendezvous_tag, solve_admission_challenge,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{BinaryType, Event, MessageEvent, WebSocket};
use zeroize::Zeroizing;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const ACK_TIMEOUT_MS: f64 = 15_000.0;
const MAX_SOCKET_BUFFER_BYTES: u32 = 256 * 1024;
const MAX_PACKET_BYTES: usize = 16 * 1024 + FRAME_HEADER_LEN;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebProgress {
    Connecting,
    WaitingForPeer,
    Handshaking,
    Connected,
}

pub(crate) enum WebCommand {
    SendText(Zeroizing<Vec<u8>>),
    Burn,
}

pub(crate) enum WebEvent {
    Delivered,
    ReceivedText(Zeroizing<String>),
    PeerBurned,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebExit {
    LocalClose,
    LocalBurn,
    PeerBurn,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebError {
    Connection,
    RelayRejected,
    Authentication,
    DeliveryTimeout,
    Cancelled,
}

enum SocketEvent {
    Open,
    Binary(Vec<u8>),
    Closed,
    Failed,
}

enum PollResult {
    Event(SocketEvent),
    TimedOut,
}

struct BrowserSocket {
    socket: WebSocket,
    events: UnboundedReceiver<SocketEvent>,
    _open: Closure<dyn FnMut(Event)>,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _error: Closure<dyn FnMut(Event)>,
    _close: Closure<dyn FnMut(Event)>,
}

impl BrowserSocket {
    fn new(url: &str) -> Result<Self, WebError> {
        let socket = WebSocket::new(url).map_err(|_| WebError::Connection)?;
        socket.set_binary_type(BinaryType::Arraybuffer);
        let (sender, events) = mpsc::unbounded();

        let open_sender = sender.clone();
        let open = Closure::wrap(Box::new(move |_: Event| {
            let _ = open_sender.unbounded_send(SocketEvent::Open);
        }) as Box<dyn FnMut(Event)>);

        let message_sender = sender.clone();
        let message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();
            if !data.is_instance_of::<ArrayBuffer>() {
                let _ = message_sender.unbounded_send(SocketEvent::Failed);
                return;
            }
            let bytes = Uint8Array::new(&data);
            let Ok(length) = usize::try_from(bytes.length()) else {
                let _ = message_sender.unbounded_send(SocketEvent::Failed);
                return;
            };
            if length > MAX_PACKET_BYTES {
                let _ = message_sender.unbounded_send(SocketEvent::Failed);
                return;
            }
            let mut packet = vec![0_u8; length];
            bytes.copy_to(&mut packet);
            let _ = message_sender.unbounded_send(SocketEvent::Binary(packet));
        }) as Box<dyn FnMut(MessageEvent)>);

        let error_sender = sender.clone();
        let error = Closure::wrap(Box::new(move |_: Event| {
            let _ = error_sender.unbounded_send(SocketEvent::Failed);
        }) as Box<dyn FnMut(Event)>);

        let close = Closure::wrap(Box::new(move |_: Event| {
            let _ = sender.unbounded_send(SocketEvent::Closed);
        }) as Box<dyn FnMut(Event)>);

        socket.set_onopen(Some(open.as_ref().unchecked_ref()));
        socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
        socket.set_onerror(Some(error.as_ref().unchecked_ref()));
        socket.set_onclose(Some(close.as_ref().unchecked_ref()));

        Ok(Self {
            socket,
            events,
            _open: open,
            _message: message,
            _error: error,
            _close: close,
        })
    }

    fn send(&self, packet: &[u8]) -> Result<(), WebError> {
        if self.socket.ready_state() != WebSocket::OPEN
            || self.socket.buffered_amount() > MAX_SOCKET_BUFFER_BYTES
            || packet.len() > MAX_PACKET_BYTES
        {
            return Err(WebError::Connection);
        }
        // The threaded FRB module uses shared Wasm memory. WebSocket.send does
        // not accept an ArrayBufferView backed by SharedArrayBuffer, so copy
        // the already-framed packet into an ordinary JS-owned Uint8Array.
        let Ok(length) = u32::try_from(packet.len()) else {
            return Err(WebError::Connection);
        };
        let outbound = Uint8Array::new_with_length(length);
        outbound.copy_from(packet);
        self.socket
            .send_with_js_u8_array(&outbound)
            .map_err(|_| WebError::Connection)
    }

    fn close(&self) {
        let _ = self.socket.close();
    }

    async fn poll(&mut self, wait: Duration) -> PollResult {
        let incoming = self.events.next().fuse();
        let delay = delay(wait).fuse();
        pin_mut!(incoming, delay);
        select! {
            event = incoming => PollResult::Event(event.unwrap_or(SocketEvent::Failed)),
            () = delay => PollResult::TimedOut,
        }
    }
}

impl Drop for BrowserSocket {
    fn drop(&mut self) {
        self.socket.set_onopen(None);
        self.socket.set_onmessage(None);
        self.socket.set_onerror(None);
        self.socket.set_onclose(None);
        let _ = self.socket.close();
    }
}

pub(crate) struct WebAuthenticatedClient {
    socket: BrowserSocket,
    handshake: HandshakeSession,
}

impl WebAuthenticatedClient {
    pub(crate) fn verification_code(&self) -> Option<String> {
        self.handshake
            .handshake_hash()
            .map(super::api::session::format_verification_code)
    }

    pub(crate) async fn run_live(
        mut self,
        mut take_command: impl FnMut() -> Option<WebCommand>,
        mut on_event: impl FnMut(WebEvent),
        cancelled: impl Fn() -> bool,
    ) -> Result<WebExit, WebError> {
        let mut pending_counter: Option<(u64, f64)> = None;
        loop {
            if cancelled() {
                self.socket.close();
                return Ok(WebExit::LocalClose);
            }
            let now = monotonic_now_ms()?;
            if pending_counter.is_some_and(|(_, deadline)| now >= deadline) {
                return Err(WebError::DeliveryTimeout);
            }
            if let Some(command) = take_command() {
                match command {
                    WebCommand::SendText(text) if pending_counter.is_none() => {
                        let frame = self
                            .handshake
                            .encrypt_transport_frame(
                                FrameKind::Ciphertext,
                                &text,
                                ProtocolLimits::default(),
                            )
                            .map_err(|_| WebError::Authentication)?;
                        let counter = frame.counter();
                        send_protocol_frame(&self.socket, &frame)?;
                        pending_counter = Some((counter, now + ACK_TIMEOUT_MS));
                    }
                    WebCommand::SendText(_) => return Err(WebError::Authentication),
                    WebCommand::Burn => {
                        let frame = self
                            .handshake
                            .encrypt_burn_frame(ProtocolLimits::default())
                            .map_err(|_| WebError::Authentication)?;
                        send_protocol_frame(&self.socket, &frame)?;
                        self.socket.close();
                        return Ok(WebExit::LocalBurn);
                    }
                }
                continue;
            }

            match self.socket.poll(POLL_INTERVAL).await {
                PollResult::TimedOut => {}
                PollResult::Event(SocketEvent::Binary(packet)) => {
                    let frame = decode_protocol_packet(&packet)?;
                    let authenticated = self
                        .handshake
                        .decrypt_transport_frame(&frame, ProtocolLimits::default())
                        .map_err(|_| WebError::Authentication)?;
                    match authenticated.kind() {
                        FrameKind::Ciphertext => {
                            let mut bytes = Zeroizing::new(authenticated.payload().to_vec());
                            let text = String::from_utf8(std::mem::take(&mut *bytes))
                                .map_err(|_| WebError::Authentication)?;
                            let ack = self
                                .handshake
                                .encrypt_transport_frame(
                                    FrameKind::Ack,
                                    &authenticated.counter().to_be_bytes(),
                                    ProtocolLimits::default(),
                                )
                                .map_err(|_| WebError::Authentication)?;
                            send_protocol_frame(&self.socket, &ack)?;
                            on_event(WebEvent::ReceivedText(Zeroizing::new(text)));
                        }
                        FrameKind::Ack => {
                            let acknowledged = authenticated
                                .acknowledged_counter()
                                .ok_or(WebError::Authentication)?;
                            if pending_counter.map(|(counter, _)| counter) != Some(acknowledged) {
                                return Err(WebError::Authentication);
                            }
                            pending_counter = None;
                            on_event(WebEvent::Delivered);
                        }
                        FrameKind::Burn => {
                            on_event(WebEvent::PeerBurned);
                            self.socket.close();
                            return Ok(WebExit::PeerBurn);
                        }
                        FrameKind::Handshake | FrameKind::Error => {
                            return Err(WebError::Authentication);
                        }
                    }
                }
                PollResult::Event(SocketEvent::Closed) => return Err(WebError::RelayRejected),
                PollResult::Event(SocketEvent::Failed | SocketEvent::Open) => {
                    return Err(WebError::Connection);
                }
            }
        }
    }
}

pub(crate) async fn authenticate_derived(
    relay_url: &str,
    room: RoomCode,
    secret: SessionSecret,
    operation_timeout: Duration,
    mut progress: impl FnMut(WebProgress),
    cancelled: impl Fn() -> bool,
) -> Result<WebAuthenticatedClient, WebError> {
    progress(WebProgress::Connecting);
    let tag = derive_rendezvous_tag(&room, &secret).map_err(|_| WebError::Authentication)?;
    let mut socket = BrowserSocket::new(relay_url)?;
    wait_for_open(&mut socket, operation_timeout, &cancelled).await?;
    socket.send(&JoinRequest { tag: tag.clone() }.encode())?;
    let role = await_pairing(
        &mut socket,
        &tag,
        operation_timeout,
        &mut progress,
        &cancelled,
    )
    .await?;
    progress(WebProgress::Handshaking);
    let handshake_role = match role {
        PeerRole::Initiator => HandshakeRole::Initiator,
        PeerRole::Responder => HandshakeRole::Responder,
    };
    let mut handshake = HandshakeSession::new(handshake_role, &room, &secret, &tag)
        .map_err(|_| WebError::Authentication)?;

    match role {
        PeerRole::Initiator => {
            let first = handshake
                .initiator_start()
                .map_err(|_| WebError::Authentication)?;
            send_handshake_frame(&socket, 0, first)?;
            let second =
                receive_handshake_frame(&mut socket, 1, operation_timeout, &cancelled).await?;
            let third = handshake
                .initiator_receive_response(second.payload())
                .map_err(|_| WebError::Authentication)?;
            send_handshake_frame(&socket, 2, third)?;
            let fourth =
                receive_handshake_frame(&mut socket, 3, operation_timeout, &cancelled).await?;
            handshake
                .initiator_receive_confirmation(fourth.payload())
                .map_err(|_| WebError::Authentication)?;
        }
        PeerRole::Responder => {
            let first =
                receive_handshake_frame(&mut socket, 0, operation_timeout, &cancelled).await?;
            let second = handshake
                .responder_receive_initial(first.payload())
                .map_err(|_| WebError::Authentication)?;
            send_handshake_frame(&socket, 1, second)?;
            let third =
                receive_handshake_frame(&mut socket, 2, operation_timeout, &cancelled).await?;
            let fourth = handshake
                .responder_receive_confirmation(third.payload())
                .map_err(|_| WebError::Authentication)?;
            send_handshake_frame(&socket, 3, fourth)?;
        }
    }

    if !handshake.is_connected() {
        return Err(WebError::Authentication);
    }
    progress(WebProgress::Connected);
    Ok(WebAuthenticatedClient { socket, handshake })
}

async fn wait_for_open(
    socket: &mut BrowserSocket,
    timeout: Duration,
    cancelled: &impl Fn() -> bool,
) -> Result<(), WebError> {
    let deadline = monotonic_now_ms()? + duration_ms(timeout);
    loop {
        if cancelled() {
            return Err(WebError::Cancelled);
        }
        if monotonic_now_ms()? >= deadline {
            return Err(WebError::Connection);
        }
        match socket.poll(POLL_INTERVAL).await {
            PollResult::TimedOut => {}
            PollResult::Event(SocketEvent::Open) => return Ok(()),
            PollResult::Event(SocketEvent::Closed | SocketEvent::Failed) => {
                return Err(WebError::Connection);
            }
            PollResult::Event(SocketEvent::Binary(_)) => return Err(WebError::Connection),
        }
    }
}

async fn await_pairing(
    socket: &mut BrowserSocket,
    tag: &RendezvousTag,
    timeout: Duration,
    progress: &mut impl FnMut(WebProgress),
    cancelled: &impl Fn() -> bool,
) -> Result<PeerRole, WebError> {
    let mut first = receive_binary(socket, timeout, cancelled).await?;
    if let RelayEvent::PuzzleChallenge(challenge) =
        RelayEvent::decode(&first).map_err(|_| WebError::RelayRejected)?
    {
        let solution = solve_admission_challenge(challenge, tag).ok_or(WebError::RelayRejected)?;
        socket.send(&PuzzleSolution { value: solution }.encode())?;
        first = receive_binary(socket, timeout, cancelled).await?;
    }
    match RelayEvent::decode(&first).map_err(|_| WebError::RelayRejected)? {
        RelayEvent::Waiting(PeerRole::Initiator) => {
            progress(WebProgress::WaitingForPeer);
            let ready = receive_binary(socket, timeout, cancelled).await?;
            match RelayEvent::decode(&ready).map_err(|_| WebError::RelayRejected)? {
                RelayEvent::PeerReady(PeerRole::Initiator) => Ok(PeerRole::Initiator),
                _ => Err(WebError::RelayRejected),
            }
        }
        RelayEvent::PeerReady(PeerRole::Responder) => Ok(PeerRole::Responder),
        _ => Err(WebError::RelayRejected),
    }
}

fn send_handshake_frame(
    socket: &BrowserSocket,
    counter: u64,
    payload: Vec<u8>,
) -> Result<(), WebError> {
    let frame = Frame::new(
        FrameKind::Handshake,
        counter,
        payload,
        ProtocolLimits::default(),
    )
    .map_err(|_| WebError::Authentication)?;
    send_protocol_frame(socket, &frame)
}

async fn receive_handshake_frame(
    socket: &mut BrowserSocket,
    expected_counter: u64,
    timeout: Duration,
    cancelled: &impl Fn() -> bool,
) -> Result<Frame, WebError> {
    let packet = receive_binary(socket, timeout, cancelled).await?;
    let frame = decode_protocol_packet(&packet)?;
    if frame.kind() != FrameKind::Handshake || frame.counter() != expected_counter {
        return Err(WebError::Authentication);
    }
    Ok(frame)
}

fn send_protocol_frame(socket: &BrowserSocket, frame: &Frame) -> Result<(), WebError> {
    let packet = frame.encode().map_err(|_| WebError::Authentication)?;
    socket.send(&packet)
}

async fn receive_binary(
    socket: &mut BrowserSocket,
    timeout: Duration,
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<u8>, WebError> {
    let deadline = monotonic_now_ms()? + duration_ms(timeout);
    loop {
        if cancelled() {
            return Err(WebError::Cancelled);
        }
        if monotonic_now_ms()? >= deadline {
            return Err(WebError::Connection);
        }
        match socket.poll(POLL_INTERVAL).await {
            PollResult::TimedOut => {}
            PollResult::Event(SocketEvent::Binary(packet)) => return Ok(packet),
            PollResult::Event(SocketEvent::Closed) => return Err(WebError::RelayRejected),
            PollResult::Event(SocketEvent::Failed | SocketEvent::Open) => {
                return Err(WebError::Connection);
            }
        }
    }
}

fn decode_protocol_packet(packet: &[u8]) -> Result<Frame, WebError> {
    if matches!(
        RelayEvent::decode(packet),
        Ok(RelayEvent::RoomClosed | RelayEvent::Error)
    ) {
        return Err(WebError::RelayRejected);
    }
    Frame::decode(packet, ProtocolLimits::default()).map_err(|_| WebError::Authentication)
}

async fn delay(duration: Duration) {
    let milliseconds = duration_ms(duration).clamp(1.0, f64::from(i32::MAX));
    let global = js_sys::global();
    let mut schedule = |resolve: js_sys::Function, _reject: js_sys::Function| {
        if let Ok(value) = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            && let Ok(set_timeout) = value.dyn_into::<js_sys::Function>()
        {
            let _ = set_timeout.call2(&global, &resolve, &JsValue::from_f64(milliseconds));
        }
    };
    let _ = JsFuture::from(Promise::new(&mut schedule)).await;
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn monotonic_now_ms() -> Result<f64, WebError> {
    let global = js_sys::global();
    let performance = js_sys::Reflect::get(&global, &JsValue::from_str("performance"))
        .map_err(|_| WebError::Connection)?;
    let now = js_sys::Reflect::get(&performance, &JsValue::from_str("now"))
        .map_err(|_| WebError::Connection)?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| WebError::Connection)?;
    now.call0(&performance)
        .map_err(|_| WebError::Connection)?
        .as_f64()
        .ok_or(WebError::Connection)
}
