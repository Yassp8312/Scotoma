use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use argon2::{Algorithm, Argon2, Params, Version};
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

pub const MIN_NUMERIC_DIGITS: usize = 1;
pub const MAX_NUMERIC_DIGITS: usize = 128;
pub const SESSION_SECRET_BYTES: usize = 32;
pub const RENDEZVOUS_TAG_BYTES: usize = 32;

const PASSWORD_SALT_CONTEXT: &[u8] = b"scotoma-password-salt-v2";
const RENDEZVOUS_CONTEXT: &[u8] = b"scotoma-rendezvous-v2";

/// Canonical numeric room label. It is not a source of cryptographic security.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomCode(String);

impl RoomCode {
    /// Parses an ASCII-decimal room and removes redundant leading zeroes.
    ///
    /// # Errors
    ///
    /// Rejects empty, non-ASCII-numeric, or overlong input.
    pub fn parse(input: &str) -> Result<Self, NumericInputError> {
        parse_numeric(input).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Debug for RoomCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("RoomCode([REDACTED])")
    }
}

impl Display for RoomCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Numeric password entered for each session and zeroized on drop.
pub struct NumericPassword(Zeroizing<String>);

impl NumericPassword {
    /// Parses an ASCII-decimal password and removes redundant leading zeroes.
    ///
    /// # Errors
    ///
    /// Rejects empty, non-ASCII-numeric, or overlong input.
    pub fn parse(input: &str) -> Result<Self, NumericInputError> {
        parse_numeric(input).map(|value| Self(Zeroizing::new(value)))
    }

    #[must_use]
    pub fn expose_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Debug for NumericPassword {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("NumericPassword([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericInputError {
    Empty,
    TooLong,
    NonNumeric,
}

impl Display for NumericInputError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value must contain at least one digit"),
            Self::TooLong => write!(
                formatter,
                "value must contain at most {MAX_NUMERIC_DIGITS} digits"
            ),
            Self::NonNumeric => formatter.write_str("value must contain only ASCII digits"),
        }
    }
}

impl Error for NumericInputError {}

fn parse_numeric(input: &str) -> Result<String, NumericInputError> {
    if input.is_empty() {
        return Err(NumericInputError::Empty);
    }
    if input.len() > MAX_NUMERIC_DIGITS {
        return Err(NumericInputError::TooLong);
    }
    if !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(NumericInputError::NonNumeric);
    }

    let canonical = input.trim_start_matches('0');
    Ok(if canonical.is_empty() {
        String::from("0")
    } else {
        canonical.to_owned()
    })
}

/// Tunable Argon2id costs. Defaults are a portable prototype baseline and must
/// be benchmarked on the weakest supported browser and phone before release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordKdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
}

impl Default for PasswordKdfParams {
    fn default() -> Self {
        Self {
            memory_kib: 8 * 1024,
            iterations: 2,
            lanes: 1,
        }
    }
}

/// Fixed-size secret derived locally from the numeric password.
pub struct SessionSecret([u8; SESSION_SECRET_BYTES]);

impl SessionSecret {
    #[must_use]
    pub const fn expose_bytes(&self) -> &[u8; SESSION_SECRET_BYTES] {
        &self.0
    }

    #[cfg(test)]
    pub(crate) const fn from_bytes_for_test(bytes: [u8; SESSION_SECRET_BYTES]) -> Self {
        Self(bytes)
    }
}

impl Debug for SessionSecret {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionSecret([REDACTED])")
    }
}

impl Drop for SessionSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyDerivationError;

impl Display for KeyDerivationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("unable to derive session key material")
    }
}

impl Error for KeyDerivationError {}

/// Derives the fixed session secret locally with Argon2id.
///
/// The deterministic salt binds the result to the canonical room. This raises
/// the cost of guesses but cannot add entropy to a short user password.
///
/// # Errors
///
/// Returns a generic error for invalid KDF parameters or failed derivation.
pub fn derive_session_secret(
    room: &RoomCode,
    password: &NumericPassword,
    settings: PasswordKdfParams,
) -> Result<SessionSecret, KeyDerivationError> {
    let params = Params::new(
        settings.memory_kib,
        settings.iterations,
        settings.lanes,
        Some(SESSION_SECRET_BYTES),
    )
    .map_err(|_| KeyDerivationError)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut salt_hasher = Sha256::new();
    salt_hasher.update(PASSWORD_SALT_CONTEXT);
    update_length_prefixed(&mut salt_hasher, room.as_str().as_bytes())?;
    let salt = salt_hasher.finalize();
    let mut output = [0_u8; SESSION_SECRET_BYTES];
    argon
        .hash_password_into(password.expose_bytes(), &salt, &mut output)
        .map_err(|_| KeyDerivationError)?;
    Ok(SessionSecret(output))
}

/// Opaque routing value visible to the relay but inappropriate for logs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RendezvousTag([u8; RENDEZVOUS_TAG_BYTES]);

impl RendezvousTag {
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

/// Derives the opaque relay routing tag from v2 key material.
///
/// # Errors
///
/// Returns a generic error if the bounded room cannot be encoded.
pub fn derive_rendezvous_tag(
    room: &RoomCode,
    secret: &SessionSecret,
) -> Result<RendezvousTag, KeyDerivationError> {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret.expose_bytes())
        .map_err(|_| KeyDerivationError)?;
    mac.update(RENDEZVOUS_CONTEXT);
    update_hmac_length_prefixed(&mut mac, room.as_str().as_bytes())?;

    let output = mac.finalize().into_bytes();
    let mut tag = [0_u8; RENDEZVOUS_TAG_BYTES];
    tag.copy_from_slice(&output);
    Ok(RendezvousTag(tag))
}

fn update_length_prefixed(digest: &mut Sha256, value: &[u8]) -> Result<(), KeyDerivationError> {
    let length = u16::try_from(value.len()).map_err(|_| KeyDerivationError)?;
    digest.update(length.to_be_bytes());
    digest.update(value);
    Ok(())
}

fn update_hmac_length_prefixed(
    mac: &mut Hmac<Sha256>,
    value: &[u8],
) -> Result<(), KeyDerivationError> {
    let length = u16::try_from(value.len()).map_err(|_| KeyDerivationError)?;
    mac.update(&length.to_be_bytes());
    mac.update(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_NUMERIC_DIGITS, NumericInputError, NumericPassword, PasswordKdfParams, RoomCode,
        derive_rendezvous_tag, derive_session_secret,
    };

    #[test]
    fn numeric_fields_accept_39_and_128_digits() {
        assert!(RoomCode::parse(&"7".repeat(39)).is_ok());
        assert!(NumericPassword::parse(&"8".repeat(39)).is_ok());
        assert!(RoomCode::parse(&"9".repeat(MAX_NUMERIC_DIGITS)).is_ok());
        assert!(NumericPassword::parse(&"1".repeat(MAX_NUMERIC_DIGITS)).is_ok());
    }

    #[test]
    fn numeric_fields_reject_unbounded_or_non_ascii_input() {
        assert_eq!(RoomCode::parse(""), Err(NumericInputError::Empty));
        assert_eq!(
            NumericPassword::parse(&"1".repeat(MAX_NUMERIC_DIGITS + 1)).unwrap_err(),
            NumericInputError::TooLong
        );
        assert_eq!(RoomCode::parse("12 34"), Err(NumericInputError::NonNumeric));
        assert_eq!(RoomCode::parse("１２"), Err(NumericInputError::NonNumeric));
    }

    #[test]
    fn leading_zeroes_are_canonical_for_room_and_password() {
        let first_room = RoomCode::parse("00013").expect("valid room");
        let second_room = RoomCode::parse("13").expect("valid room");
        let first_password = NumericPassword::parse("001370").expect("valid password");
        let second_password = NumericPassword::parse("1370").expect("valid password");
        let settings = PasswordKdfParams {
            memory_kib: 8,
            iterations: 1,
            lanes: 1,
        };
        let first = derive_session_secret(&first_room, &first_password, settings).expect("KDF");
        let second = derive_session_secret(&second_room, &second_password, settings).expect("KDF");

        assert_eq!(first_room, second_room);
        assert_eq!(first.expose_bytes(), second.expose_bytes());
    }

    #[test]
    fn same_inputs_produce_same_tag_and_changes_separate_rooms() {
        let room = RoomCode::parse("13").expect("valid room");
        let other_room = RoomCode::parse("14").expect("valid room");
        let password = NumericPassword::parse("1370").expect("valid password");
        let settings = PasswordKdfParams {
            memory_kib: 8,
            iterations: 1,
            lanes: 1,
        };
        let first = derive_session_secret(&room, &password, settings).expect("KDF");
        let second_password = NumericPassword::parse("1370").expect("valid password");
        let second = derive_session_secret(&room, &second_password, settings).expect("KDF");
        let different_password = NumericPassword::parse("1346").expect("valid password");
        let decoy = derive_session_secret(&room, &different_password, settings).expect("KDF");
        let other_password = NumericPassword::parse("1370").expect("valid password");
        let other = derive_session_secret(&other_room, &other_password, settings).expect("KDF");

        assert_eq!(
            derive_rendezvous_tag(&room, &first).expect("tag"),
            derive_rendezvous_tag(&room, &second).expect("tag")
        );
        assert_ne!(
            derive_rendezvous_tag(&room, &first).expect("tag"),
            derive_rendezvous_tag(&other_room, &other).expect("tag")
        );
        assert_ne!(
            derive_rendezvous_tag(&room, &first).expect("tag"),
            derive_rendezvous_tag(&room, &decoy).expect("tag")
        );
    }

    #[test]
    fn secrets_are_redacted() {
        let password = NumericPassword::parse("1370").expect("valid password");
        let room = RoomCode::parse("13").expect("valid room");
        let secret = derive_session_secret(
            &room,
            &password,
            PasswordKdfParams {
                memory_kib: 8,
                iterations: 1,
                lanes: 1,
            },
        )
        .expect("KDF");

        assert_eq!(format!("{password:?}"), "NumericPassword([REDACTED])");
        assert_eq!(format!("{room:?}"), "RoomCode([REDACTED])");
        assert_eq!(format!("{secret:?}"), "SessionSecret([REDACTED])");
    }
}
