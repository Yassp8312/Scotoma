//! Protocol boundaries shared by SCOTOMA clients, the simulator and the relay.
//!
//! It defines bounded wire frames, invite derivation and the authenticated
//! Noise handshake and application transport used by SCOTOMA clients.

mod admission;
mod frame;
mod invite;
mod limits;
mod noise_handshake;
mod relay_wire;
mod state;

pub use admission::{
    AdmissionChallenge, MAX_PUZZLE_DIFFICULTY_BITS, PUZZLE_NONCE_BYTES, solve_admission_challenge,
    verify_admission_solution,
};
pub use frame::{FRAME_HEADER_LEN, Frame, FrameKind, ProtocolError};
pub use invite::{
    KeyDerivationError, MAX_NUMERIC_DIGITS, MIN_NUMERIC_DIGITS, NumericInputError, NumericPassword,
    PasswordKdfParams, RENDEZVOUS_TAG_BYTES, RendezvousTag, RoomCode, SESSION_SECRET_BYTES,
    SessionSecret, derive_rendezvous_tag, derive_session_secret,
};
pub use limits::ProtocolLimits;
pub use noise_handshake::{
    AuthenticatedFrame, HandshakeError, HandshakeRole, HandshakeSession, HandshakeStage,
    NOISE_PROTOCOL_NAME, TransportError,
};
pub use relay_wire::{JoinRequest, PeerRole, PuzzleSolution, RelayEvent, RelayWireError};
pub use state::{SessionEvent, SessionState, TransitionError};

/// Wire protocol version supported by this build.
pub const PROTOCOL_VERSION: u8 = 2;
