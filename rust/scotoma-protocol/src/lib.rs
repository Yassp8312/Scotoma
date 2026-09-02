//! Protocol boundaries shared by SCOTOMA clients, the simulator and the relay.
//!
//! It defines bounded wire frames, invite derivation and the authenticated
//! Noise handshake and application transport used by SCOTOMA clients.

mod frame;
mod invite;
mod limits;
mod noise_handshake;
mod relay_wire;
mod state;

pub use frame::{FRAME_HEADER_LEN, Frame, FrameKind, ProtocolError};
pub use invite::{
    INVITE_SECRET_BYTES, InviteSecret, MAX_ROOM_NUMBER, MIN_ROOM_NUMBER, RENDEZVOUS_TAG_BYTES,
    RendezvousError, RendezvousTag, RoomCode, RoomCodeError, derive_rendezvous_tag,
};
pub use limits::ProtocolLimits;
pub use noise_handshake::{
    AuthenticatedFrame, HandshakeError, HandshakeRole, HandshakeSession, HandshakeStage,
    NOISE_PROTOCOL_NAME, TransportError,
};
pub use relay_wire::{JoinRequest, PeerRole, RelayEvent, RelayWireError};
pub use state::{SessionEvent, SessionState, TransitionError};

/// Wire protocol version supported by this build.
pub const PROTOCOL_VERSION: u8 = 1;
