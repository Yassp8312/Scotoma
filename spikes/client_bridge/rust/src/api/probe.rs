//! A real, public-only call into the existing protocol crate.

/// Public protocol metadata, never caller-controlled or secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolProbe {
    pub protocol_version: u8,
    pub min_credential_digits: u32,
    pub max_credential_digits: u32,
    pub noise_protocol: String,
}

/// Deliberately takes no arguments: credentials cannot cross this probe API.
pub fn protocol_probe() -> ProtocolProbe {
    ProtocolProbe {
        protocol_version: scotoma_protocol::PROTOCOL_VERSION,
        min_credential_digits: scotoma_protocol::MIN_NUMERIC_DIGITS as u32,
        max_credential_digits: scotoma_protocol::MAX_NUMERIC_DIGITS as u32,
        noise_protocol: scotoma_protocol::NOISE_PROTOCOL_NAME.to_owned(),
    }
}

/// Exercises the platform CSPRNG without exposing random bytes over the bridge.
/// A successful call is an availability check, not an entropy-quality audit.
pub fn randomness_probe() -> bool {
    let mut bytes = zeroize::Zeroizing::new([0_u8; 32]);
    getrandom::fill(&mut *bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::protocol_probe;

    #[test]
    fn operating_system_randomness_is_available() {
        assert!(super::randomness_probe());
    }

    #[test]
    fn reports_existing_core_metadata_without_credentials() {
        let probe = protocol_probe();
        assert_eq!(probe.protocol_version, 2);
        assert_eq!(probe.min_credential_digits, 1);
        assert_eq!(probe.max_credential_digits, 128);
        assert_eq!(probe.noise_protocol, scotoma_protocol::NOISE_PROTOCOL_NAME);
    }
}
