//! CLI client plumbing for the SCOTOMA protocol prototype.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use scotoma_protocol::{
    Frame, FrameKind, HandshakeRole, HandshakeSession, InviteSecret, JoinRequest, PeerRole,
    ProtocolLimits, RelayEvent, RoomCode, derive_rendezvous_tag,
};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Connection inputs for one authenticated CLI client.
pub struct ClientConfig {
    pub relay_url: String,
    pub room: RoomCode,
    pub secret: InviteSecret,
    pub operation_timeout: Duration,
}

impl Debug for ClientConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientConfig")
            .field("relay_url", &self.relay_url)
            .field("room", &self.room)
            .field("secret", &"[REDACTED]")
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
    InvalidInviteInput,
}

impl Display for ClientError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection => formatter.write_str("relay connection failed"),
            Self::RelayRejected => formatter.write_str("relay session failed"),
            Self::Authentication => formatter.write_str("peer authentication failed"),
            Self::InvalidInviteInput => formatter.write_str("invite secret must be 64 hex digits"),
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
    let tag = derive_rendezvous_tag(config.room, &config.secret)
        .map_err(|_| ClientError::Authentication)?;
    let connect = timeout(config.operation_timeout, connect_async(&config.relay_url))
        .await
        .map_err(|_| ClientError::Connection)?
        .map_err(|_| ClientError::Connection)?;
    let (mut socket, _) = connect;

    send_binary(
        &mut socket,
        JoinRequest { tag: tag.clone() }.encode(),
        config.operation_timeout,
    )
    .await?;
    let role = await_pairing(&mut socket, config.operation_timeout).await?;
    let handshake_role = match role {
        PeerRole::Initiator => HandshakeRole::Initiator,
        PeerRole::Responder => HandshakeRole::Responder,
    };
    let mut handshake = HandshakeSession::new(handshake_role, config.room, &config.secret, &tag)
        .map_err(|_| ClientError::Authentication)?;

    match role {
        PeerRole::Initiator => {
            let first = handshake
                .initiator_start()
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 0, first, config.operation_timeout).await?;
            let second = receive_frame(&mut socket, 1, config.operation_timeout).await?;
            let third = handshake
                .initiator_receive_response(second.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 2, third, config.operation_timeout).await?;
            let fourth = receive_frame(&mut socket, 3, config.operation_timeout).await?;
            handshake
                .initiator_receive_confirmation(fourth.payload())
                .map_err(|_| ClientError::Authentication)?;
        }
        PeerRole::Responder => {
            let first = receive_frame(&mut socket, 0, config.operation_timeout).await?;
            let second = handshake
                .responder_receive_initial(first.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 1, second, config.operation_timeout).await?;
            let third = receive_frame(&mut socket, 2, config.operation_timeout).await?;
            let fourth = handshake
                .responder_receive_confirmation(third.payload())
                .map_err(|_| ClientError::Authentication)?;
            send_frame(&mut socket, 3, fourth, config.operation_timeout).await?;
        }
    }

    if !handshake.is_connected() {
        return Err(ClientError::Authentication);
    }

    Ok(AuthenticatedClient {
        role,
        socket,
        handshake,
        operation_timeout: config.operation_timeout,
    })
}

/// Decodes the temporary hexadecimal invite representation used by the CLI.
///
/// # Errors
///
/// Rejects values that are not exactly 32 hexadecimal bytes.
pub fn decode_invite_secret(input: &str) -> Result<InviteSecret, ClientError> {
    let input = input.trim().as_bytes();
    if input.len() != 64 {
        return Err(ClientError::InvalidInviteInput);
    }

    let mut bytes = [0_u8; 32];
    for (index, pair) in input.chunks_exact(2).enumerate() {
        let high = decode_nibble(*pair.first().ok_or(ClientError::InvalidInviteInput)?)?;
        let low = decode_nibble(*pair.get(1).ok_or(ClientError::InvalidInviteInput)?)?;
        let target = bytes
            .get_mut(index)
            .ok_or(ClientError::InvalidInviteInput)?;
        *target = (high << 4) | low;
    }
    Ok(InviteSecret::from_bytes(bytes))
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
    operation_timeout: Duration,
) -> Result<PeerRole, ClientError> {
    let first = receive_binary(socket, operation_timeout).await?;
    match RelayEvent::decode(&first).map_err(|_| ClientError::RelayRejected)? {
        RelayEvent::Waiting(PeerRole::Initiator) => {
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
    if matches!(
        RelayEvent::decode(&encoded),
        Ok(RelayEvent::RoomClosed | RelayEvent::Error)
    ) {
        return Err(ClientError::RelayRejected);
    }
    let frame = Frame::decode(&encoded, ProtocolLimits::default())
        .map_err(|_| ClientError::Authentication)?;
    Ok(frame)
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

fn decode_nibble(value: u8) -> Result<u8, ClientError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(ClientError::InvalidInviteInput),
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientError, decode_invite_secret, encode_hex};

    #[test]
    fn invite_hex_round_trips_expected_bytes() {
        let encoded = "42".repeat(32);
        let secret = decode_invite_secret(&encoded).expect("valid secret");

        assert_eq!(secret.expose_bytes(), &[0x42; 32]);
    }

    #[test]
    fn invalid_invite_input_is_rejected() {
        assert!(matches!(
            decode_invite_secret("not-a-secret"),
            Err(ClientError::InvalidInviteInput)
        ));
        assert!(matches!(
            decode_invite_secret(&"zz".repeat(32)),
            Err(ClientError::InvalidInviteInput)
        ));
    }

    #[test]
    fn hex_encoder_is_canonical_lowercase() {
        assert_eq!(encode_hex(&[0x00, 0xab, 0xff]), "00abff");
    }
}
