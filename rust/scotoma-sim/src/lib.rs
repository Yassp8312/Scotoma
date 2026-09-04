//! Reusable live client plumbing for the SCOTOMA protocol prototype.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::{
    Frame, FrameKind, HandshakeRole, HandshakeSession, JoinRequest, NumericPassword,
    PasswordKdfParams, PeerRole, ProtocolLimits, PuzzleSolution, RelayEvent, RendezvousTag,
    RoomCode, SessionSecret, derive_rendezvous_tag, derive_session_secret,
    solve_admission_challenge,
};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use zeroize::Zeroizing;

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub const DEFAULT_ACK_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy)]
struct AckDeadline(tokio::time::Instant);

impl AckDeadline {
    fn after(duration: Duration) -> Self {
        Self(tokio::time::Instant::now() + duration)
    }

    async fn wait(self) {
        tokio::time::sleep_until(self.0).await;
    }
}

/// Connection inputs for one authenticated CLI client.
pub struct ClientConfig {
    pub relay_url: String,
    pub room: RoomCode,
    pub password: NumericPassword,
    pub operation_timeout: Duration,
}

/// Already-derived connection inputs. The password never crosses this boundary.
pub struct DerivedClientConfig {
    pub relay_url: String,
    pub room: RoomCode,
    pub secret: SessionSecret,
    pub operation_timeout: Duration,
}

impl Debug for DerivedClientConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DerivedClientConfig")
            .field("relay_url", &self.relay_url)
            .field("room", &"[REDACTED]")
            .field("secret", &"[REDACTED]")
            .field("operation_timeout", &self.operation_timeout)
            .finish_non_exhaustive()
    }
}

/// Coarse, non-secret progress visible to the application UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientProgress {
    Connecting,
    WaitingForPeer,
    Handshaking,
    Connected,
}

/// Bounded commands accepted by an authenticated live session.
pub enum LiveCommand {
    SendText(Zeroizing<Vec<u8>>),
    Burn,
}

impl Debug for LiveCommand {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendText(text) => formatter
                .debug_tuple("SendText")
                .field(&format_args!("[REDACTED; {} bytes]", text.len()))
                .finish(),
            Self::Burn => formatter.write_str("Burn"),
        }
    }
}

/// Authenticated events produced by a live session.
pub enum LiveEvent {
    Delivered(u64),
    ReceivedText(Zeroizing<String>),
    PeerBurned,
}

impl Debug for LiveEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Delivered(counter) => formatter.debug_tuple("Delivered").field(counter).finish(),
            Self::ReceivedText(text) => formatter
                .debug_tuple("ReceivedText")
                .field(&format_args!("[REDACTED; {} bytes]", text.len()))
                .finish(),
            Self::PeerBurned => formatter.write_str("PeerBurned"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveExit {
    LocalClose,
    LocalBurn,
    PeerBurn,
}

impl Debug for ClientConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientConfig")
            .field("relay_url", &self.relay_url)
            .field("room", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .field("operation_timeout", &self.operation_timeout)
            .finish_non_exhaustive()
    }
}

/// A live socket whose peer completed mutual Noise key confirmation.
pub struct AuthenticatedClient {
    role: PeerRole,
    socket: ClientSocket,
    handshake: HandshakeSession,
    operation_timeout: Duration,
}

impl AuthenticatedClient {
    #[must_use]
    pub const fn role(&self) -> PeerRole {
        self.role
    }

    /// Returns the confirmed channel-binding hash.
    #[must_use]
    pub fn handshake_hash(&self) -> Option<&[u8; 32]> {
        self.handshake.handshake_hash()
    }

    /// Sends one encrypted text message and waits for its authenticated ACK.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] if encryption, transport, authentication or ACK
    /// validation fails.
    pub async fn send_text_and_wait_ack(&mut self, text: &str) -> Result<u64, ClientError> {
        let frame = self
            .handshake
            .encrypt_transport_frame(
                FrameKind::Ciphertext,
                text.as_bytes(),
                ProtocolLimits::default(),
            )
            .map_err(|_| ClientError::Authentication)?;
        let sent_counter = frame.counter();
        send_protocol_frame(&mut self.socket, &frame, self.operation_timeout).await?;

        let ack = receive_protocol_frame(&mut self.socket, self.operation_timeout).await?;
        let authenticated = self
            .handshake
            .decrypt_transport_frame(&ack, ProtocolLimits::default())
            .map_err(|_| ClientError::Authentication)?;
        if authenticated.kind() != FrameKind::Ack
            || authenticated.acknowledged_counter() != Some(sent_counter)
        {
            return Err(ClientError::Authentication);
        }
        Ok(sent_counter)
    }

    /// Receives one encrypted UTF-8 text message and returns an authenticated ACK.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] if transport, authentication, UTF-8 validation
    /// or ACK transmission fails.
    pub async fn receive_text_and_ack(&mut self) -> Result<String, ClientError> {
        let frame = receive_protocol_frame(&mut self.socket, self.operation_timeout).await?;
        let authenticated = self
            .handshake
            .decrypt_transport_frame(&frame, ProtocolLimits::default())
            .map_err(|_| ClientError::Authentication)?;
        if authenticated.kind() != FrameKind::Ciphertext {
            return Err(ClientError::Authentication);
        }
        let text = std::str::from_utf8(authenticated.payload())
            .map_err(|_| ClientError::Authentication)?
            .to_owned();
        let ack = self
            .handshake
            .encrypt_transport_frame(
                FrameKind::Ack,
                &authenticated.counter().to_be_bytes(),
                ProtocolLimits::default(),
            )
            .map_err(|_| ClientError::Authentication)?;
        send_protocol_frame(&mut self.socket, &ack, self.operation_timeout).await?;
        Ok(text)
    }

    /// Sends an authenticated BURN, destroys local keys and closes the socket.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the terminal frame cannot be created or
    /// handed to the relay. Local state is already destroyed after creation.
    pub async fn burn(mut self) -> Result<(), ClientError> {
        let burn = self
            .handshake
            .encrypt_burn_frame(ProtocolLimits::default())
            .map_err(|_| ClientError::Authentication)?;
        let result = send_protocol_frame(&mut self.socket, &burn, self.operation_timeout).await;
        let _ = self.socket.send(Message::Close(None)).await;
        result
    }

    /// Waits for and authenticates the peer's terminal BURN frame.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] if the relay closes first or the BURN frame does
    /// not authenticate under the current ordered transport state.
    pub async fn wait_for_burn(mut self) -> Result<(), ClientError> {
        let burn = receive_protocol_frame(&mut self.socket, self.operation_timeout).await?;
        let authenticated = self
            .handshake
            .decrypt_transport_frame(&burn, ProtocolLimits::default())
            .map_err(|_| ClientError::Authentication)?;
        if authenticated.kind() != FrameKind::Burn {
            return Err(ClientError::Authentication);
        }
        let _ = self.socket.send(Message::Close(None)).await;
        Ok(())
    }

    /// Closes the development socket cleanly.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] if the close frame cannot be sent.
    pub async fn close(mut self) -> Result<(), ClientError> {
        self.socket
            .send(Message::Close(None))
            .await
            .map_err(|_| ClientError::Connection)
    }

    /// Keeps the authenticated socket alive until local cancellation or remote close.
    ///
    /// # Errors
    ///
    /// Returns a non-secret transport category if the peer closes or sends an
    /// application frame before the message layer has been enabled.
    pub async fn hold_until_cancelled(
        mut self,
        cancelled: impl Fn() -> bool,
    ) -> Result<(), ClientError> {
        loop {
            if cancelled() {
                let _ = self.socket.send(Message::Close(None)).await;
                return Ok(());
            }
            tokio::select! {
                () = tokio::time::sleep(Duration::from_millis(50)) => {}
                incoming = self.socket.next() => match incoming {
                    Some(Ok(Message::Ping(payload))) => {
                        self.socket.send(Message::Pong(payload)).await
                            .map_err(|_| ClientError::Connection)?;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => return Err(ClientError::RelayRejected),
                    Some(Ok(Message::Binary(_) | Message::Text(_))) => {
                        return Err(ClientError::Authentication);
                    }
                    Some(Ok(Message::Frame(_)) | Err(_)) => {
                        return Err(ClientError::Connection);
                    }
                }
            }
        }
    }

    /// Runs bidirectional authenticated messaging until close or BURN.
    ///
    /// At most one locally-sent ciphertext may await an ACK. Received text is
    /// acknowledged before it is emitted to the caller.
    ///
    /// # Errors
    ///
    /// Returns a generic transport category for malformed, unauthenticated,
    /// out-of-order or failed socket operations.
    pub async fn run_live(
        self,
        commands: tokio::sync::mpsc::Receiver<LiveCommand>,
        on_event: impl FnMut(LiveEvent),
        cancelled: impl Fn() -> bool,
    ) -> Result<LiveExit, ClientError> {
        self.run_live_with_ack_timeout(commands, on_event, cancelled, DEFAULT_ACK_TIMEOUT)
            .await
    }

    /// Runs the live loop with an explicit ACK deadline, primarily for bounded
    /// platform policy and deterministic tests. Timed-out ciphertext is never
    /// retransmitted automatically.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::DeliveryTimeout`] when the current local message
    /// is not acknowledged before `ack_timeout`, or another generic transport
    /// category for socket/authentication failure.
    pub async fn run_live_with_ack_timeout(
        mut self,
        mut commands: tokio::sync::mpsc::Receiver<LiveCommand>,
        mut on_event: impl FnMut(LiveEvent),
        cancelled: impl Fn() -> bool,
        ack_timeout: Duration,
    ) -> Result<LiveExit, ClientError> {
        if ack_timeout.is_zero() {
            return Err(ClientError::DeliveryTimeout);
        }
        let mut pending_counter: Option<(u64, AckDeadline)> = None;
        loop {
            if cancelled() {
                let _ = self.socket.send(Message::Close(None)).await;
                return Ok(LiveExit::LocalClose);
            }
            let pending_deadline = pending_counter.map(|(_, deadline)| deadline);
            let ack_wait = async move {
                if let Some(deadline) = pending_deadline {
                    deadline.wait().await;
                } else {
                    std::future::pending::<()>().await;
                }
            };
            tokio::select! {
                () = tokio::time::sleep(Duration::from_millis(50)) => {}
                () = ack_wait => return Err(ClientError::DeliveryTimeout),
                command = commands.recv() => match command {
                    Some(LiveCommand::SendText(text)) if pending_counter.is_none() => {
                        let frame = self.handshake.encrypt_transport_frame(
                            FrameKind::Ciphertext,
                            &text,
                            ProtocolLimits::default(),
                        ).map_err(|_| ClientError::Authentication)?;
                        let counter = frame.counter();
                        send_protocol_frame(&mut self.socket, &frame, self.operation_timeout).await?;
                        pending_counter = Some((counter, AckDeadline::after(ack_timeout)));
                    }
                    Some(LiveCommand::SendText(_)) => return Err(ClientError::Authentication),
                    Some(LiveCommand::Burn) => {
                        let burn = self.handshake.encrypt_burn_frame(ProtocolLimits::default())
                            .map_err(|_| ClientError::Authentication)?;
                        send_protocol_frame(&mut self.socket, &burn, self.operation_timeout).await?;
                        let _ = self.socket.send(Message::Close(None)).await;
                        return Ok(LiveExit::LocalBurn);
                    }
                    None => {
                        let _ = self.socket.send(Message::Close(None)).await;
                        return Ok(LiveExit::LocalClose);
                    }
                },
                incoming = self.socket.next() => match incoming {
                    Some(Ok(Message::Binary(packet))) => {
                        let frame = decode_protocol_packet(&packet)?;
                        let authenticated = self.handshake
                            .decrypt_transport_frame(&frame, ProtocolLimits::default())
                            .map_err(|_| ClientError::Authentication)?;
                        match authenticated.kind() {
                            FrameKind::Ciphertext => {
                                let mut bytes = Zeroizing::new(authenticated.payload().to_vec());
                                let text = String::from_utf8(std::mem::take(&mut *bytes))
                                    .map_err(|_| ClientError::Authentication)?;
                                let ack = self.handshake.encrypt_transport_frame(
                                    FrameKind::Ack,
                                    &authenticated.counter().to_be_bytes(),
                                    ProtocolLimits::default(),
                                ).map_err(|_| ClientError::Authentication)?;
                                send_protocol_frame(&mut self.socket, &ack, self.operation_timeout).await?;
                                on_event(LiveEvent::ReceivedText(Zeroizing::new(text)));
                            }
                            FrameKind::Ack => {
                                let acknowledged = authenticated.acknowledged_counter()
                                    .ok_or(ClientError::Authentication)?;
                                if pending_counter.map(|(counter, _)| counter) != Some(acknowledged) {
                                    return Err(ClientError::Authentication);
                                }
                                pending_counter = None;
                                on_event(LiveEvent::Delivered(acknowledged));
                            }
                            FrameKind::Burn => {
                                on_event(LiveEvent::PeerBurned);
                                let _ = self.socket.send(Message::Close(None)).await;
                                return Ok(LiveExit::PeerBurn);
                            }
                            FrameKind::Handshake | FrameKind::Error => {
                                return Err(ClientError::Authentication);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        self.socket.send(Message::Pong(payload)).await
                            .map_err(|_| ClientError::Connection)?;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => return Err(ClientError::RelayRejected),
                    Some(Ok(Message::Text(_) | Message::Frame(_)) | Err(_)) => {
                        return Err(ClientError::Connection);
                    }
                }
            }
        }
    }
}

impl Debug for AuthenticatedClient {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticatedClient")
            .field("role", &self.role)
            .field("session", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Generic client failure categories that never contain invite material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    Connection,
    RelayRejected,
    Authentication,
    DeliveryTimeout,
    InvalidInviteInput,
}

impl Display for ClientError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection => formatter.write_str("relay connection failed"),
            Self::RelayRejected => formatter.write_str("relay session failed"),
            Self::Authentication => formatter.write_str("peer authentication failed"),
            Self::DeliveryTimeout => formatter.write_str("message delivery timed out"),
            Self::InvalidInviteInput => {
                formatter.write_str("room and password must be 1 to 128 ASCII digits")
            }
        }
    }
}

impl Error for ClientError {}

/// Connects, rendezvous and completes mutual Noise key confirmation.
///
/// # Errors
///
/// Returns a non-secret-bearing [`ClientError`] for transport, relay or
/// authentication failure.
pub async fn authenticate(config: ClientConfig) -> Result<AuthenticatedClient, ClientError> {
    let ClientConfig {
        relay_url,
        room,
        password,
        operation_timeout,
    } = config;
    let secret = derive_session_secret(&room, &password, PasswordKdfParams::default())
        .map_err(|_| ClientError::Authentication)?;
    drop(password);
    authenticate_derived(
        DerivedClientConfig {
            relay_url,
            room,
            secret,
            operation_timeout,
        },
        |_| {},
    )
    .await
}

/// Connects using Rust-owned, already-derived material and reports only coarse progress.
///
/// # Errors
///
/// Returns a non-secret failure category for connection, relay or Noise errors.
pub async fn authenticate_derived(
    config: DerivedClientConfig,
    mut progress: impl FnMut(ClientProgress),
) -> Result<AuthenticatedClient, ClientError> {
    let DerivedClientConfig {
        relay_url,
        room,
        secret,
        operation_timeout,
    } = config;
    progress(ClientProgress::Connecting);
    let tag = derive_rendezvous_tag(&room, &secret).map_err(|_| ClientError::Authentication)?;
    let connect = timeout(operation_timeout, connect_async(&relay_url))
        .await
        .map_err(|_| ClientError::Connection)?
        .map_err(|_| ClientError::Connection)?;
    let (mut socket, _) = connect;

    send_binary(
        &mut socket,
        JoinRequest { tag: tag.clone() }.encode(),
        operation_timeout,
    )
    .await?;
    let role = await_pairing(&mut socket, &tag, operation_timeout, &mut progress).await?;
    progress(ClientProgress::Handshaking);
    let handshake_role = match role {
        PeerRole::Initiator => HandshakeRole::Initiator,
        PeerRole::Responder => HandshakeRole::Responder,
    };
    let mut handshake = HandshakeSession::new(handshake_role, &room, &secret, &tag)
        .map_err(|_| ClientError::Authentication)?;

    match role {
        PeerRole::Initiator => {
            let first = handshake
                .initiator_start()
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 0, first, operation_timeout).await?;
            let second = receive_frame(&mut socket, 1, operation_timeout).await?;
            let third = handshake
                .initiator_receive_response(second.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 2, third, operation_timeout).await?;
            let fourth = receive_frame(&mut socket, 3, operation_timeout).await?;
            handshake
                .initiator_receive_confirmation(fourth.payload())
                .map_err(|_| ClientError::Authentication)?;
        }
        PeerRole::Responder => {
            let first = receive_frame(&mut socket, 0, operation_timeout).await?;
            let second = handshake
                .responder_receive_initial(first.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 1, second, operation_timeout).await?;
            let third = receive_frame(&mut socket, 2, operation_timeout).await?;
            let fourth = handshake
                .responder_receive_confirmation(third.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 3, fourth, operation_timeout).await?;
        }
    }

    if !handshake.is_connected() {
        return Err(ClientError::Authentication);
    }

    progress(ClientProgress::Connected);

    Ok(AuthenticatedClient {
        role,
        socket,
        handshake,
        operation_timeout,
    })
}

/// Encodes bytes using the temporary lowercase hexadecimal CLI representation.
#[must_use]
pub fn encode_hex(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

async fn await_pairing(
    socket: &mut ClientSocket,
    tag: &RendezvousTag,
    operation_timeout: Duration,
    progress: &mut impl FnMut(ClientProgress),
) -> Result<PeerRole, ClientError> {
    let mut first = receive_binary(socket, operation_timeout).await?;
    if let RelayEvent::PuzzleChallenge(challenge) =
        RelayEvent::decode(&first).map_err(|_| ClientError::RelayRejected)?
    {
        let challenge_tag = tag.clone();
        let solution = tokio::task::spawn_blocking(move || {
            solve_admission_challenge(challenge, &challenge_tag)
        })
        .await
        .map_err(|_| ClientError::Connection)?
        .ok_or(ClientError::RelayRejected)?;
        send_binary(
            socket,
            PuzzleSolution { value: solution }.encode(),
            operation_timeout,
        )
        .await?;
        first = receive_binary(socket, operation_timeout).await?;
    }
    match RelayEvent::decode(&first).map_err(|_| ClientError::RelayRejected)? {
        RelayEvent::Waiting(PeerRole::Initiator) => {
            progress(ClientProgress::WaitingForPeer);
            let ready = receive_binary(socket, operation_timeout).await?;
            match RelayEvent::decode(&ready).map_err(|_| ClientError::RelayRejected)? {
                RelayEvent::PeerReady(PeerRole::Initiator) => Ok(PeerRole::Initiator),
                _ => Err(ClientError::RelayRejected),
            }
        }
        RelayEvent::PeerReady(PeerRole::Responder) => Ok(PeerRole::Responder),
        _ => Err(ClientError::RelayRejected),
    }
}

async fn send_frame(
    socket: &mut ClientSocket,
    counter: u64,
    payload: Vec<u8>,
    operation_timeout: Duration,
) -> Result<(), ClientError> {
    let frame = Frame::new(
        FrameKind::Handshake,
        counter,
        payload,
        ProtocolLimits::default(),
    )
    .map_err(|_| ClientError::Authentication)?;
    let encoded = frame.encode().map_err(|_| ClientError::Authentication)?;
    send_binary(socket, encoded, operation_timeout).await
}

async fn receive_frame(
    socket: &mut ClientSocket,
    expected_counter: u64,
    operation_timeout: Duration,
) -> Result<Frame, ClientError> {
    let frame = receive_protocol_frame(socket, operation_timeout).await?;
    if frame.kind() != FrameKind::Handshake || frame.counter() != expected_counter {
        return Err(ClientError::Authentication);
    }
    Ok(frame)
}

async fn send_protocol_frame(
    socket: &mut ClientSocket,
    frame: &Frame,
    operation_timeout: Duration,
) -> Result<(), ClientError> {
    let encoded = frame.encode().map_err(|_| ClientError::Authentication)?;
    send_binary(socket, encoded, operation_timeout).await
}

async fn receive_protocol_frame(
    socket: &mut ClientSocket,
    operation_timeout: Duration,
) -> Result<Frame, ClientError> {
    let encoded = receive_binary(socket, operation_timeout).await?;
    decode_protocol_packet(&encoded)
}

fn decode_protocol_packet(encoded: &[u8]) -> Result<Frame, ClientError> {
    if matches!(
        RelayEvent::decode(encoded),
        Ok(RelayEvent::RoomClosed | RelayEvent::Error)
    ) {
        return Err(ClientError::RelayRejected);
    }
    Frame::decode(encoded, ProtocolLimits::default()).map_err(|_| ClientError::Authentication)
}

async fn send_binary(
    socket: &mut ClientSocket,
    packet: Vec<u8>,
    operation_timeout: Duration,
) -> Result<(), ClientError> {
    timeout(
        operation_timeout,
        socket.send(Message::Binary(packet.into())),
    )
    .await
    .map_err(|_| ClientError::Connection)?
    .map_err(|_| ClientError::Connection)
}

async fn receive_binary(
    socket: &mut ClientSocket,
    operation_timeout: Duration,
) -> Result<Vec<u8>, ClientError> {
    let incoming = timeout(operation_timeout, socket.next())
        .await
        .map_err(|_| ClientError::Connection)?
        .ok_or(ClientError::Connection)?
        .map_err(|_| ClientError::Connection)?;
    match incoming {
        Message::Binary(packet) => Ok(packet.to_vec()),
        Message::Close(_) => Err(ClientError::RelayRejected),
        _ => Err(ClientError::Connection),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AckDeadline, encode_hex};

    #[test]
    fn hex_encoder_is_canonical_lowercase() {
        assert_eq!(encode_hex(&[0x00, 0xab, 0xff]), "00abff");
    }

    #[tokio::test(start_paused = true)]
    async fn ack_deadline_uses_the_monotonic_runtime_clock() {
        let waiter = tokio::spawn(AckDeadline::after(Duration::from_secs(15)).wait());

        tokio::time::advance(Duration::from_secs(14)).await;
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        tokio::time::advance(Duration::from_secs(1)).await;
        waiter.await.expect("deadline task must finish");
    }
}
