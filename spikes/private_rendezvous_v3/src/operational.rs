//! Bounded operational-policy model for the isolated v3 research candidate.
//!
//! This module does not authenticate infrastructure, implement a proof-of-work
//! puzzle or choose production limits. It makes the fail-closed invariants and
//! the measurements still required for R02/R03 executable.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const NETWORK_TAG_DOMAIN: &[u8] = b"SCOTOMA-v3-network-tag";
const CHALLENGE_DOMAIN: &[u8] = b"SCOTOMA-v3-admission-challenge";
const MAX_MODEL_OPERATORS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ControlDomain(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperatorDomain {
    pub operator_id: u16,
    pub infrastructure: ControlDomain,
    pub administrator: ControlDomain,
    pub credential_root: ControlDomain,
    pub backup_custodian: ControlDomain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentTopology {
    pub threshold: u16,
    pub relay_infrastructure: ControlDomain,
    pub relay_administrator: ControlDomain,
    pub operators: Vec<OperatorDomain>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyError {
    InvalidThreshold,
    TooManyOperators,
    DuplicateOperator,
    SharedInfrastructure,
    SharedAdministrator,
    SharedCredentialRoot,
    SharedBackupCustodian,
    RelaySharesControl,
}

impl DeploymentTopology {
    /// Validates declarations supplied by operators. This cannot prove that the
    /// real organizations, accounts or people are genuinely independent.
    pub fn validate(&self) -> Result<(), TopologyError> {
        if self.operators.is_empty()
            || self.threshold == 0
            || usize::from(self.threshold) > self.operators.len()
        {
            return Err(TopologyError::InvalidThreshold);
        }
        if self.operators.len() > MAX_MODEL_OPERATORS {
            return Err(TopologyError::TooManyOperators);
        }

        let mut ids = BTreeSet::new();
        let mut infrastructure = BTreeSet::new();
        let mut administrators = BTreeSet::new();
        let mut credential_roots = BTreeSet::new();
        let mut backup_custodians = BTreeSet::new();

        for operator in &self.operators {
            if !ids.insert(operator.operator_id) {
                return Err(TopologyError::DuplicateOperator);
            }
            if operator.infrastructure == self.relay_infrastructure
                || operator.administrator == self.relay_administrator
            {
                return Err(TopologyError::RelaySharesControl);
            }
            if !infrastructure.insert(operator.infrastructure) {
                return Err(TopologyError::SharedInfrastructure);
            }
            if !administrators.insert(operator.administrator) {
                return Err(TopologyError::SharedAdministrator);
            }
            if !credential_roots.insert(operator.credential_root) {
                return Err(TopologyError::SharedCredentialRoot);
            }
            if !backup_custodians.insert(operator.backup_custodian) {
                return Err(TopologyError::SharedBackupCustodian);
            }
        }
        Ok(())
    }

    pub fn compromise_action(&self, compromised: &BTreeSet<u16>) -> CompromiseAction {
        if compromised.is_empty() {
            return CompromiseAction::Continue;
        }
        let remaining = self
            .operators
            .iter()
            .filter(|operator| !compromised.contains(&operator.operator_id))
            .count();
        if remaining >= usize::from(self.threshold) {
            CompromiseAction::RotateAndReissueManifest
        } else {
            CompromiseAction::HaltWithoutThresholdReduction
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompromiseAction {
    Continue,
    RotateAndReissueManifest,
    HaltWithoutThresholdReduction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionConfig {
    pub operator_id: u16,
    pub epoch: u64,
    pub normal_attempts_per_connection: u16,
    pub network_attempts_per_window: u32,
    pub global_attempts_per_window: u32,
    pub max_connections: usize,
    pub max_networks: usize,
    pub max_challenges: usize,
    pub base_difficulty: u8,
    pub max_difficulty: u8,
    pub challenge_ttl_ticks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionConfigError {
    ZeroBound,
    InvalidDifficulty,
    InvalidCapacity,
}

impl AdmissionConfig {
    fn validate(self) -> Result<Self, AdmissionConfigError> {
        if self.normal_attempts_per_connection == 0
            || self.network_attempts_per_window == 0
            || self.global_attempts_per_window == 0
            || self.max_connections == 0
            || self.max_networks == 0
            || self.max_challenges == 0
            || self.challenge_ttl_ticks == 0
        {
            return Err(AdmissionConfigError::ZeroBound);
        }
        if self.base_difficulty == 0
            || self.base_difficulty > self.max_difficulty
            || self.max_difficulty > 63
        {
            return Err(AdmissionConfigError::InvalidDifficulty);
        }
        if self.network_attempts_per_window > self.global_attempts_per_window {
            return Err(AdmissionConfigError::InvalidCapacity);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PuzzleProof {
    pub challenge_id: [u8; 32],
    /// Result of a local verifier. Never deserialize this boolean from a peer.
    pub verified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionDecision {
    Allow,
    Challenge {
        challenge_id: [u8; 32],
        difficulty: u8,
        expires_at: u64,
    },
    /// Every pre-admission rejection maps to this single wire-level result.
    RejectUniform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingChallenge {
    connection_id: u64,
    network_tag: [u8; 32],
    expires_at: u64,
}

#[derive(Clone, Debug)]
pub struct AdmissionController {
    config: AdmissionConfig,
    window_id: u64,
    privacy_key: [u8; 32],
    connection_attempts: BTreeMap<u64, u16>,
    network_attempts: BTreeMap<[u8; 32], u32>,
    challenges: BTreeMap<[u8; 32], PendingChallenge>,
    global_attempts: u32,
    challenge_nonce: u64,
}

impl AdmissionController {
    pub fn new(
        config: AdmissionConfig,
        window_id: u64,
        privacy_key: [u8; 32],
    ) -> Result<Self, AdmissionConfigError> {
        let config = config.validate()?;
        if privacy_key == [0; 32] {
            return Err(AdmissionConfigError::ZeroBound);
        }
        Ok(Self {
            config,
            window_id,
            privacy_key,
            connection_attempts: BTreeMap::new(),
            network_attempts: BTreeMap::new(),
            challenges: BTreeMap::new(),
            global_attempts: 0,
            challenge_nonce: 0,
        })
    }

    pub fn admit(
        &mut self,
        now: u64,
        connection_id: u64,
        raw_network_prefix: &[u8],
        proof: Option<PuzzleProof>,
    ) -> AdmissionDecision {
        self.expire_challenges(now);
        if raw_network_prefix.is_empty()
            || self.global_attempts >= self.config.global_attempts_per_window
        {
            return AdmissionDecision::RejectUniform;
        }

        let network_tag = self.network_tag(raw_network_prefix);
        if (!self.connection_attempts.contains_key(&connection_id)
            && self.connection_attempts.len() >= self.config.max_connections)
            || (!self.network_attempts.contains_key(&network_tag)
                && self.network_attempts.len() >= self.config.max_networks)
        {
            return AdmissionDecision::RejectUniform;
        }

        let connection_attempts = self
            .connection_attempts
            .get(&connection_id)
            .copied()
            .unwrap_or(0);
        let network_attempts = self
            .network_attempts
            .get(&network_tag)
            .copied()
            .unwrap_or(0);
        let challenge_required = connection_attempts >= self.config.normal_attempts_per_connection
            || network_attempts >= self.config.network_attempts_per_window;

        if let Some(proof) = proof {
            let valid = self.consume_proof(now, connection_id, network_tag, proof);
            if !challenge_required || !valid {
                return AdmissionDecision::RejectUniform;
            }
        } else if challenge_required {
            return self.issue_challenge(
                now,
                connection_id,
                network_tag,
                connection_attempts,
                network_attempts,
            );
        }

        self.connection_attempts
            .insert(connection_id, connection_attempts.saturating_add(1));
        self.network_attempts
            .insert(network_tag, network_attempts.saturating_add(1));
        self.global_attempts = self.global_attempts.saturating_add(1);
        AdmissionDecision::Allow
    }

    pub fn rotate_window(
        &mut self,
        new_window_id: u64,
        new_privacy_key: [u8; 32],
    ) -> Result<(), AdmissionConfigError> {
        if new_window_id <= self.window_id
            || new_privacy_key == [0; 32]
            || new_privacy_key == self.privacy_key
        {
            return Err(AdmissionConfigError::InvalidCapacity);
        }
        self.window_id = new_window_id;
        self.privacy_key = new_privacy_key;
        self.connection_attempts.clear();
        self.network_attempts.clear();
        self.challenges.clear();
        self.global_attempts = 0;
        self.challenge_nonce = 0;
        Ok(())
    }

    pub fn tracked_connections(&self) -> usize {
        self.connection_attempts.len()
    }

    pub fn tracked_networks(&self) -> usize {
        self.network_attempts.len()
    }

    pub fn pending_challenges(&self) -> usize {
        self.challenges.len()
    }

    pub fn admitted_in_window(&self) -> u32 {
        self.global_attempts
    }

    fn issue_challenge(
        &mut self,
        now: u64,
        connection_id: u64,
        network_tag: [u8; 32],
        connection_attempts: u16,
        network_attempts: u32,
    ) -> AdmissionDecision {
        if self.challenges.len() >= self.config.max_challenges {
            return AdmissionDecision::RejectUniform;
        }
        let pressure = u64::from(connection_attempts)
            .saturating_add(u64::from(network_attempts))
            .saturating_sub(u64::from(self.config.normal_attempts_per_connection));
        let difficulty = self
            .config
            .base_difficulty
            .saturating_add(u8::try_from(pressure.min(u64::from(u8::MAX))).unwrap_or(u8::MAX))
            .min(self.config.max_difficulty);
        let expires_at = now.saturating_add(self.config.challenge_ttl_ticks);
        let challenge_id = self.challenge_id(connection_id, network_tag, difficulty, expires_at);
        self.challenge_nonce = self.challenge_nonce.saturating_add(1);
        self.challenges.insert(
            challenge_id,
            PendingChallenge {
                connection_id,
                network_tag,
                expires_at,
            },
        );
        AdmissionDecision::Challenge {
            challenge_id,
            difficulty,
            expires_at,
        }
    }

    fn consume_proof(
        &mut self,
        now: u64,
        connection_id: u64,
        network_tag: [u8; 32],
        proof: PuzzleProof,
    ) -> bool {
        let Some(challenge) = self.challenges.remove(&proof.challenge_id) else {
            return false;
        };
        proof.verified
            && challenge.connection_id == connection_id
            && challenge.network_tag == network_tag
            && now <= challenge.expires_at
    }

    fn expire_challenges(&mut self, now: u64) {
        self.challenges
            .retain(|_, challenge| now <= challenge.expires_at);
    }

    fn network_tag(&self, raw_network_prefix: &[u8]) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(NETWORK_TAG_DOMAIN);
        hash.update(self.privacy_key);
        hash.update((raw_network_prefix.len() as u64).to_be_bytes());
        hash.update(raw_network_prefix);
        hash.finalize().into()
    }

    fn challenge_id(
        &self,
        connection_id: u64,
        network_tag: [u8; 32],
        difficulty: u8,
        expires_at: u64,
    ) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(CHALLENGE_DOMAIN);
        hash.update(self.privacy_key);
        hash.update(self.config.operator_id.to_be_bytes());
        hash.update(self.config.epoch.to_be_bytes());
        hash.update(self.window_id.to_be_bytes());
        hash.update(connection_id.to_be_bytes());
        hash.update(network_tag);
        hash.update([difficulty]);
        hash.update(expires_at.to_be_bytes());
        hash.update(self.challenge_nonce.to_be_bytes());
        hash.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operator(id: u16, base: u64) -> OperatorDomain {
        OperatorDomain {
            operator_id: id,
            infrastructure: ControlDomain(base),
            administrator: ControlDomain(base + 10),
            credential_root: ControlDomain(base + 20),
            backup_custodian: ControlDomain(base + 30),
        }
    }

    fn topology() -> DeploymentTopology {
        DeploymentTopology {
            threshold: 2,
            relay_infrastructure: ControlDomain(90),
            relay_administrator: ControlDomain(91),
            operators: vec![operator(1, 1), operator(2, 2), operator(3, 3)],
        }
    }

    fn config() -> AdmissionConfig {
        AdmissionConfig {
            operator_id: 7,
            epoch: 11,
            normal_attempts_per_connection: 2,
            network_attempts_per_window: 4,
            global_attempts_per_window: 8,
            max_connections: 8,
            max_networks: 8,
            max_challenges: 2,
            base_difficulty: 4,
            max_difficulty: 12,
            challenge_ttl_ticks: 5,
        }
    }

    fn controller() -> AdmissionController {
        AdmissionController::new(config(), 1, [0x44; 32]).unwrap()
    }

    #[test]
    fn independent_declared_topology_and_compromise_actions_are_fail_closed() {
        let topology = topology();
        assert_eq!(topology.validate(), Ok(()));
        assert_eq!(
            topology.compromise_action(&BTreeSet::new()),
            CompromiseAction::Continue
        );
        assert_eq!(
            topology.compromise_action(&BTreeSet::from([1])),
            CompromiseAction::RotateAndReissueManifest
        );
        assert_eq!(
            topology.compromise_action(&BTreeSet::from([1, 2])),
            CompromiseAction::HaltWithoutThresholdReduction
        );
    }

    #[test]
    fn shared_control_domains_are_rejected() {
        let mut shared_provider = topology();
        shared_provider.operators[1].infrastructure = shared_provider.operators[0].infrastructure;
        assert_eq!(
            shared_provider.validate(),
            Err(TopologyError::SharedInfrastructure)
        );

        let mut shared_admin = topology();
        shared_admin.operators[1].administrator = shared_admin.operators[0].administrator;
        assert_eq!(
            shared_admin.validate(),
            Err(TopologyError::SharedAdministrator)
        );

        let mut shared_credentials = topology();
        shared_credentials.operators[1].credential_root =
            shared_credentials.operators[0].credential_root;
        assert_eq!(
            shared_credentials.validate(),
            Err(TopologyError::SharedCredentialRoot)
        );

        let mut shared_backup = topology();
        shared_backup.operators[1].backup_custodian = shared_backup.operators[0].backup_custodian;
        assert_eq!(
            shared_backup.validate(),
            Err(TopologyError::SharedBackupCustodian)
        );
    }

    #[test]
    fn relay_cannot_control_an_operator() {
        let mut deployment = topology();
        deployment.operators[0].administrator = deployment.relay_administrator;
        assert_eq!(
            deployment.validate(),
            Err(TopologyError::RelaySharesControl)
        );
    }

    #[test]
    fn first_two_attempts_are_fast_then_a_bound_challenge_is_required() {
        let mut admission = controller();
        let network = b"shared-prefix";
        assert_eq!(
            admission.admit(10, 1, network, None),
            AdmissionDecision::Allow
        );
        assert_eq!(
            admission.admit(11, 1, network, None),
            AdmissionDecision::Allow
        );
        let AdmissionDecision::Challenge { challenge_id, .. } =
            admission.admit(12, 1, network, None)
        else {
            panic!("third attempt must be challenged");
        };
        assert_eq!(
            admission.admit(
                13,
                1,
                network,
                Some(PuzzleProof {
                    challenge_id,
                    verified: true
                })
            ),
            AdmissionDecision::Allow
        );
        assert_eq!(admission.pending_challenges(), 0);
    }

    #[test]
    fn challenge_is_single_use_and_bound_to_connection_network_and_expiry() {
        let cases: &[(u64, &[u8], u64, bool)] = &[
            (2, b"network-a", 13, true),
            (1, b"network-b", 13, true),
            (1, b"network-a", 18, true),
            (1, b"network-a", 13, false),
        ];
        for &(connection, network, now, verified) in cases {
            let mut admission = controller();
            admission.admit(10, 1, b"network-a", None);
            admission.admit(11, 1, b"network-a", None);
            let AdmissionDecision::Challenge { challenge_id, .. } =
                admission.admit(12, 1, b"network-a", None)
            else {
                panic!("challenge expected");
            };
            assert_eq!(
                admission.admit(
                    now,
                    connection,
                    network,
                    Some(PuzzleProof {
                        challenge_id,
                        verified
                    })
                ),
                AdmissionDecision::RejectUniform
            );
            assert_eq!(admission.pending_challenges(), 0);
        }
    }

    #[test]
    fn shared_nat_can_challenge_a_new_connection() {
        let mut admission = controller();
        for connection in 1..=4 {
            assert_eq!(
                admission.admit(1, connection, b"same-nat", None),
                AdmissionDecision::Allow
            );
        }
        assert!(matches!(
            admission.admit(1, 5, b"same-nat", None),
            AdmissionDecision::Challenge { .. }
        ));
    }

    #[test]
    fn distributed_sources_are_still_stopped_by_global_capacity() {
        let mut admission = controller();
        for index in 0..8_u8 {
            assert_eq!(
                admission.admit(1, u64::from(index), &[index.saturating_add(1)], None),
                AdmissionDecision::Allow
            );
        }
        assert_eq!(
            admission.admit(1, 99, b"another-network", None),
            AdmissionDecision::RejectUniform
        );
        assert_eq!(admission.admitted_in_window(), 8);
    }

    #[test]
    fn state_and_outstanding_challenges_are_strictly_bounded() {
        let mut admission = controller();
        for connection in 1..=8 {
            let network = [connection as u8];
            assert_eq!(
                admission.admit(1, connection, &network, None),
                AdmissionDecision::Allow
            );
        }
        assert_eq!(admission.tracked_connections(), 8);
        assert_eq!(admission.tracked_networks(), 8);
        assert_eq!(
            admission.admit(1, 9, b"ninth", None),
            AdmissionDecision::RejectUniform
        );

        let mut challenges = controller();
        for connection in 1..=2 {
            challenges.admit(1, connection, b"net", None);
            challenges.admit(1, connection, b"net", None);
            assert!(matches!(
                challenges.admit(1, connection, b"net", None),
                AdmissionDecision::Challenge { .. }
            ));
        }
        assert_eq!(challenges.pending_challenges(), 2);
        assert_eq!(
            challenges.admit(1, 3, b"net", None),
            AdmissionDecision::RejectUniform
        );
    }

    #[test]
    fn rotation_forgets_ephemeral_profiles_and_invalidates_challenges() {
        let mut admission = controller();
        admission.admit(1, 1, b"network", None);
        admission.admit(1, 1, b"network", None);
        let AdmissionDecision::Challenge { challenge_id, .. } =
            admission.admit(1, 1, b"network", None)
        else {
            panic!("challenge expected");
        };

        admission.rotate_window(2, [0x55; 32]).unwrap();
        assert_eq!(admission.tracked_connections(), 0);
        assert_eq!(admission.tracked_networks(), 0);
        assert_eq!(admission.pending_challenges(), 0);
        assert_eq!(admission.admitted_in_window(), 0);
        assert_eq!(
            admission.admit(
                2,
                1,
                b"network",
                Some(PuzzleProof {
                    challenge_id,
                    verified: true
                })
            ),
            AdmissionDecision::RejectUniform
        );
    }
}
