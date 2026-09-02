use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroize;

/// SCOTOMA invite secrets contain 256 random bits.
pub const INVITE_SECRET_BYTES: usize = 32;
/// Rendezvous tags are full HMAC-SHA-256 outputs.
pub const RENDEZVOUS_TAG_BYTES: usize = 32;
pub const MIN_ROOM_NUMBER: u64 = 1;
pub const MAX_ROOM_NUMBER: u64 = 999_999_999_999;

const RENDEZVOUS_CONTEXT: &[u8] = b"scotoma-rendezvous-v1";

/// Human-readable room label. It is not a source of cryptographic security.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomCode(u64);

impl RoomCode {
    /// Creates a canonical numeric room code.
    ///
    /// # Errors
    ///
    /// Returns [`RoomCodeError`] when the value is outside the supported range.
    pub const fn new(value: u64) -> Result<Self, RoomCodeError> {
        if value < MIN_ROOM_NUMBER || value > MAX_ROOM_NUMBER {
            return Err(RoomCodeError { value });
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl Display for RoomCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Invalid human-readable room number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomCodeError {
    pub value: u64,
}

impl Display for RoomCodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "room number must be between {MIN_ROOM_NUMBER} and {MAX_ROOM_NUMBER}"
        )
    }
}

impl Error for RoomCodeError {}

/// High-entropy secret shared out of band by the two intended participants.
pub struct InviteSecret([u8; INVITE_SECRET_BYTES]);

impl InviteSecret {
    /// Generates a secret using the operating system's cryptographic RNG.
    ///
    /// # Errors
    ///
    /// Returns the platform RNG error if secure random bytes are unavailable.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; INVITE_SECRET_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// Constructs a secret from decoded invite bytes.
    ///
    /// The caller is responsible for ensuring these bytes came from a secure
    /// generated invite rather than a human password.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; INVITE_SECRET_BYTES]) -> Self {
        Self(bytes)
    }

    /// Exposes the bytes only for invite encoding and cryptographic operations.
    #[must_use]
    pub const fn expose_bytes(&self) -> &[u8; INVITE_SECRET_BYTES] {
        &self.0
    }
}

impl Debug for InviteSecret {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("InviteSecret([REDACTED])")
    }
}

impl Drop for InviteSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Opaque routing value visible to the relay but inappropriate for logs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RendezvousTag([u8; RENDEZVOUS_TAG_BYTES]);

impl RendezvousTag {
    /// Reconstructs a tag received by the relay in a join packet.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; RENDEZVOUS_TAG_BYTES]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; RENDEZVOUS_TAG_BYTES] {
        &self.0
    }
}

impl Debug for RendezvousTag {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("RendezvousTag([REDACTED])")
    }
}

/// Failure to initialize the HMAC used for a rendezvous tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendezvousError;

impl Display for RendezvousError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("unable to initialize rendezvous derivation")
    }
}

impl Error for RendezvousError {}

/// Derives a domain-separated opaque tag from a room and generated invite secret.
///
/// # Errors
///
/// Returns [`RendezvousError`] if the HMAC implementation rejects the key.
pub fn derive_rendezvous_tag(
    room: RoomCode,
    secret: &InviteSecret,
) -> Result<RendezvousTag, RendezvousError> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret.expose_bytes())
        .map_err(|_| RendezvousError)?;
    mac.update(RENDEZVOUS_CONTEXT);
    mac.update(&[0]);
    mac.update(&room.value().to_be_bytes());

    let output = mac.finalize().into_bytes();
    let mut tag = [0_u8; RENDEZVOUS_TAG_BYTES];
    tag.copy_from_slice(&output);
    Ok(RendezvousTag(tag))
}

#[cfg(test)]
mod tests {
    use super::{
        INVITE_SECRET_BYTES, InviteSecret, MAX_ROOM_NUMBER, MIN_ROOM_NUMBER, RoomCode,
        derive_rendezvous_tag,
    };

    #[test]
    fn room_range_is_enforced() {
        assert!(RoomCode::new(MIN_ROOM_NUMBER).is_ok());
        assert!(RoomCode::new(MAX_ROOM_NUMBER).is_ok());
        assert!(RoomCode::new(MIN_ROOM_NUMBER - 1).is_err());
        assert!(RoomCode::new(MAX_ROOM_NUMBER + 1).is_err());
    }

    #[test]
    fn generated_secret_has_the_expected_size_and_is_not_zero() {
        let secret = InviteSecret::generate().expect("operating system RNG should be available");

        assert_eq!(secret.expose_bytes().len(), INVITE_SECRET_BYTES);
        assert_ne!(secret.expose_bytes(), &[0; INVITE_SECRET_BYTES]);
    }

    #[test]
    fn secret_debug_output_is_redacted() {
        let secret = InviteSecret::from_bytes([0xA5; INVITE_SECRET_BYTES]);

        assert_eq!(format!("{secret:?}"), "InviteSecret([REDACTED])");
    }

    #[test]
    fn rendezvous_tag_debug_output_is_redacted() {
        let secret = InviteSecret::from_bytes([0xA5; INVITE_SECRET_BYTES]);
        let room = RoomCode::new(3).expect("room should be valid");
        let tag = derive_rendezvous_tag(room, &secret).expect("HMAC accepts this key size");

        assert_eq!(format!("{tag:?}"), "RendezvousTag([REDACTED])");
    }

    #[test]
    fn same_invite_produces_the_same_tag() {
        let secret_bytes = [0x42; INVITE_SECRET_BYTES];
        let first_secret = InviteSecret::from_bytes(secret_bytes);
        let second_secret = InviteSecret::from_bytes(secret_bytes);
        let room = RoomCode::new(3).expect("room should be valid");

        let first = derive_rendezvous_tag(room, &first_secret).expect("HMAC accepts this key size");
        let second =
            derive_rendezvous_tag(room, &second_secret).expect("HMAC accepts this key size");

        assert_eq!(first, second);
    }

    #[test]
    fn changing_room_or_secret_changes_the_tag() {
        let first_secret = InviteSecret::from_bytes([0x11; INVITE_SECRET_BYTES]);
        let second_secret = InviteSecret::from_bytes([0x22; INVITE_SECRET_BYTES]);
        let room_three = RoomCode::new(3).expect("room should be valid");
        let room_four = RoomCode::new(4).expect("room should be valid");

        let baseline =
            derive_rendezvous_tag(room_three, &first_secret).expect("HMAC accepts this key size");
        let other_room =
            derive_rendezvous_tag(room_four, &first_secret).expect("HMAC accepts this key size");
        let other_secret =
            derive_rendezvous_tag(room_three, &second_secret).expect("HMAC accepts this key size");

        assert_ne!(baseline, other_room);
        assert_ne!(baseline, other_secret);
    }

    #[test]
    fn rendezvous_derivation_matches_independent_test_vector() {
        let secret = InviteSecret::from_bytes([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);
        let room = RoomCode::new(3).expect("room should be valid");
        let expected = [
            0x94, 0xe4, 0xd5, 0x79, 0x8e, 0x94, 0x90, 0xca, 0xcd, 0xa9, 0xea, 0x59, 0x7c, 0x8d,
            0x91, 0x00, 0xe5, 0x13, 0xa2, 0x47, 0x54, 0x75, 0x68, 0x74, 0x12, 0xe7, 0x60, 0x5a,
            0x2a, 0x13, 0x02, 0x84,
        ];

        let tag = derive_rendezvous_tag(room, &secret).expect("HMAC accepts this key size");

        assert_eq!(tag.as_bytes(), &expected);
    }
}
