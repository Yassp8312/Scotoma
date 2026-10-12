//! Authenticated epoch-overlap and session-pinning model for v3 research.
//!
//! This module consumes manifests that already passed `ContinuityState`. It
//! does not authenticate manifests or provide durable platform storage.

use crate::manifest::ContinuityState;

/// Maximum time in which relay-visible traffic from a previously confirmed
/// epoch may coexist with new attempts in its authenticated successor.
pub const MAX_EPOCH_OVERLAP_SECONDS: u64 = 300;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedEpoch {
    pub sequence: u64,
    pub epoch: u64,
    pub manifest_hash: [u8; 32],
    pub valid_from: u64,
    pub valid_until: u64,
    pub previous_manifest_hash: [u8; 32],
}

impl AuthenticatedEpoch {
    pub fn from_continuity(state: &ContinuityState) -> Result<Self, EpochError> {
        let body = state
            .accepted()
            .ok_or(EpochError::UnauthenticatedManifest)?;
        Ok(Self {
            sequence: body.sequence,
            epoch: body.epoch,
            manifest_hash: state
                .accepted_hash()
                .ok_or(EpochError::UnauthenticatedManifest)?,
            valid_from: body.valid_from,
            valid_until: body.valid_until,
            previous_manifest_hash: body.previous_manifest_hash,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpochTransition {
    previous: AuthenticatedEpoch,
    current: AuthenticatedEpoch,
    overlap_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionEpochPhase {
    Provisional,
    Confirmed,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpochAbortReason {
    RotationBoundary,
    EpochExpired,
    WrongEpoch,
    WrongManifest,
    InfrastructureFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionEpochGuard {
    pinned: AuthenticatedEpoch,
    phase: SessionEpochPhase,
    abort_reason: Option<EpochAbortReason>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpochError {
    UnauthenticatedManifest,
    NonConsecutiveSequence,
    NonConsecutiveEpoch,
    WrongPreviousManifest,
    InvalidValidityProgression,
    AvailabilityGap,
    ExcessiveOverlap,
    NoValidEpoch,
    WrongPhase,
    Aborted(EpochAbortReason),
}

impl EpochTransition {
    pub fn new(
        previous: AuthenticatedEpoch,
        current: AuthenticatedEpoch,
    ) -> Result<Self, EpochError> {
        let next_sequence = previous
            .sequence
            .checked_add(1)
            .ok_or(EpochError::NonConsecutiveSequence)?;
        if current.sequence != next_sequence {
            return Err(EpochError::NonConsecutiveSequence);
        }
        let next_epoch = previous
            .epoch
            .checked_add(1)
            .ok_or(EpochError::NonConsecutiveEpoch)?;
        if current.epoch != next_epoch {
            return Err(EpochError::NonConsecutiveEpoch);
        }
        if current.previous_manifest_hash != previous.manifest_hash {
            return Err(EpochError::WrongPreviousManifest);
        }
        if current.valid_from <= previous.valid_from
            || current.valid_until <= previous.valid_until
            || current.valid_from >= current.valid_until
        {
            return Err(EpochError::InvalidValidityProgression);
        }
        if current.valid_from > previous.valid_until {
            return Err(EpochError::AvailabilityGap);
        }
        let overlap_seconds = previous.valid_until - current.valid_from;
        if overlap_seconds > MAX_EPOCH_OVERLAP_SECONDS {
            return Err(EpochError::ExcessiveOverlap);
        }
        Ok(Self {
            previous,
            current,
            overlap_seconds,
        })
    }

    pub fn from_continuity(
        previous: &ContinuityState,
        current: &ContinuityState,
    ) -> Result<Self, EpochError> {
        Self::new(
            AuthenticatedEpoch::from_continuity(previous)?,
            AuthenticatedEpoch::from_continuity(current)?,
        )
    }

    pub const fn previous(&self) -> AuthenticatedEpoch {
        self.previous
    }

    pub const fn current(&self) -> AuthenticatedEpoch {
        self.current
    }

    pub const fn overlap_seconds(&self) -> u64 {
        self.overlap_seconds
    }

    /// Selects exactly one epoch. Once the new epoch becomes valid, new
    /// attempts never fall back to the previous epoch, even if the new
    /// operators are unavailable.
    pub fn epoch_for_new_attempt(&self, now: u64) -> Result<AuthenticatedEpoch, EpochError> {
        if now >= self.current.valid_from && now < self.current.valid_until {
            Ok(self.current)
        } else if now >= self.previous.valid_from && now < self.current.valid_from {
            Ok(self.previous)
        } else {
            Err(EpochError::NoValidEpoch)
        }
    }

    pub fn begin(&self, now: u64) -> Result<SessionEpochGuard, EpochError> {
        Ok(SessionEpochGuard {
            pinned: self.epoch_for_new_attempt(now)?,
            phase: SessionEpochPhase::Provisional,
            abort_reason: None,
        })
    }
}

impl SessionEpochGuard {
    pub const fn pinned(&self) -> AuthenticatedEpoch {
        self.pinned
    }

    pub const fn phase(&self) -> SessionEpochPhase {
        self.phase
    }

    pub const fn abort_reason(&self) -> Option<EpochAbortReason> {
        self.abort_reason
    }

    pub fn validate_response(
        &mut self,
        manifest_hash: [u8; 32],
        epoch: u64,
    ) -> Result<(), EpochError> {
        self.require_provisional()?;
        if manifest_hash != self.pinned.manifest_hash {
            return self.abort(EpochAbortReason::WrongManifest);
        }
        if epoch != self.pinned.epoch {
            return self.abort(EpochAbortReason::WrongEpoch);
        }
        Ok(())
    }

    pub fn observe_time(
        &mut self,
        transition: &EpochTransition,
        now: u64,
    ) -> Result<(), EpochError> {
        if self.phase == SessionEpochPhase::Terminal {
            return Err(EpochError::WrongPhase);
        }
        if self.pinned == transition.previous {
            if self.phase == SessionEpochPhase::Provisional && now >= transition.current.valid_from
            {
                return self.abort(EpochAbortReason::RotationBoundary);
            }
            if now >= transition.previous.valid_until {
                return self.abort(EpochAbortReason::EpochExpired);
            }
        } else if self.pinned == transition.current && now >= transition.current.valid_until {
            return self.abort(EpochAbortReason::EpochExpired);
        }
        Ok(())
    }

    pub fn confirm(&mut self) -> Result<(), EpochError> {
        self.require_provisional()?;
        self.phase = SessionEpochPhase::Confirmed;
        Ok(())
    }

    pub fn infrastructure_failure(&mut self) -> Result<(), EpochError> {
        if self.phase == SessionEpochPhase::Terminal {
            return Err(EpochError::WrongPhase);
        }
        self.abort(EpochAbortReason::InfrastructureFailure)
    }

    fn require_provisional(&mut self) -> Result<(), EpochError> {
        if self.phase != SessionEpochPhase::Provisional {
            return Err(EpochError::WrongPhase);
        }
        Ok(())
    }

    fn abort<T>(&mut self, reason: EpochAbortReason) -> Result<T, EpochError> {
        self.phase = SessionEpochPhase::Terminal;
        self.abort_reason = Some(reason);
        Err(EpochError::Aborted(reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(
        sequence: u64,
        epoch: u64,
        hash_byte: u8,
        previous_hash_byte: u8,
        valid_from: u64,
        valid_until: u64,
    ) -> AuthenticatedEpoch {
        AuthenticatedEpoch {
            sequence,
            epoch,
            manifest_hash: [hash_byte; 32],
            valid_from,
            valid_until,
            previous_manifest_hash: [previous_hash_byte; 32],
        }
    }

    fn transition() -> EpochTransition {
        EpochTransition::new(
            epoch(7, 41, 0x41, 0x40, 100, 220),
            epoch(8, 42, 0x42, 0x41, 200, 320),
        )
        .unwrap()
    }

    #[test]
    fn overlap_is_bounded_and_requires_an_exact_authenticated_chain() {
        let valid = transition();
        assert_eq!(valid.overlap_seconds(), 20);

        let previous = valid.previous();
        let current = valid.current();
        let mut broken = current;
        broken.previous_manifest_hash = [0x99; 32];
        assert_eq!(
            EpochTransition::new(previous, broken),
            Err(EpochError::WrongPreviousManifest)
        );
        broken = current;
        broken.epoch += 1;
        assert_eq!(
            EpochTransition::new(previous, broken),
            Err(EpochError::NonConsecutiveEpoch)
        );
        broken = current;
        broken.valid_from = 221;
        assert_eq!(
            EpochTransition::new(previous, broken),
            Err(EpochError::AvailabilityGap)
        );
        let long_previous = epoch(7, 41, 0x41, 0x40, 100, 1_000);
        broken = epoch(8, 42, 0x42, 0x41, 699, 1_100);
        assert_eq!(
            EpochTransition::new(long_previous, broken),
            Err(EpochError::ExcessiveOverlap)
        );
    }

    #[test]
    fn new_attempts_switch_once_and_never_query_both_epochs() {
        let transition = transition();
        assert_eq!(transition.epoch_for_new_attempt(199).unwrap().epoch, 41);
        assert_eq!(transition.epoch_for_new_attempt(200).unwrap().epoch, 42);
        assert_eq!(transition.epoch_for_new_attempt(219).unwrap().epoch, 42);
        assert_eq!(
            transition.epoch_for_new_attempt(320),
            Err(EpochError::NoValidEpoch)
        );
    }

    #[test]
    fn exhausted_sequence_and_epoch_cannot_have_a_successor() {
        let current = epoch(8, 42, 0x42, 0x41, 200, 320);
        let max_sequence = epoch(u64::MAX, 41, 0x41, 0x40, 100, 220);
        assert_eq!(
            EpochTransition::new(max_sequence, current),
            Err(EpochError::NonConsecutiveSequence)
        );

        let max_epoch = epoch(7, u64::MAX, 0x41, 0x40, 100, 220);
        assert_eq!(
            EpochTransition::new(max_epoch, current),
            Err(EpochError::NonConsecutiveEpoch)
        );
    }

    #[test]
    fn provisional_old_epoch_is_terminal_at_cutover_and_cannot_resume() {
        let transition = transition();
        let mut session = transition.begin(199).unwrap();
        assert_eq!(
            session.observe_time(&transition, 200),
            Err(EpochError::Aborted(EpochAbortReason::RotationBoundary))
        );
        assert_eq!(session.phase(), SessionEpochPhase::Terminal);
        assert_eq!(session.confirm(), Err(EpochError::WrongPhase));
        assert_eq!(
            session.validate_response([0x41; 32], 41),
            Err(EpochError::WrongPhase)
        );
    }

    #[test]
    fn confirmed_old_epoch_is_pinned_only_until_its_authenticated_expiry() {
        let transition = transition();
        let mut session = transition.begin(199).unwrap();
        session.confirm().unwrap();
        session.observe_time(&transition, 219).unwrap();
        assert_eq!(session.pinned().epoch, 41);
        assert_eq!(session.phase(), SessionEpochPhase::Confirmed);
        assert_eq!(
            session.observe_time(&transition, 220),
            Err(EpochError::Aborted(EpochAbortReason::EpochExpired))
        );
    }

    #[test]
    fn mixed_manifest_or_epoch_is_terminal_before_confirmation() {
        let transition = transition();
        let mut wrong_manifest = transition.begin(200).unwrap();
        assert_eq!(
            wrong_manifest.validate_response([0x41; 32], 42),
            Err(EpochError::Aborted(EpochAbortReason::WrongManifest))
        );
        let mut wrong_epoch = transition.begin(200).unwrap();
        assert_eq!(
            wrong_epoch.validate_response([0x42; 32], 41),
            Err(EpochError::Aborted(EpochAbortReason::WrongEpoch))
        );
    }

    #[test]
    fn current_epoch_failure_is_terminal_without_previous_epoch_fallback() {
        let transition = transition();
        let mut failed = transition.begin(205).unwrap();
        assert_eq!(failed.pinned().epoch, 42);
        assert_eq!(
            failed.infrastructure_failure(),
            Err(EpochError::Aborted(EpochAbortReason::InfrastructureFailure))
        );
        assert_eq!(failed.phase(), SessionEpochPhase::Terminal);

        let retry = transition.begin(206).unwrap();
        assert_eq!(retry.pinned().epoch, 42);
    }
}
