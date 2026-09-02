use sha2::{Digest, Sha256};

use crate::RendezvousTag;

pub const PUZZLE_NONCE_BYTES: usize = 16;
pub const MAX_PUZZLE_DIFFICULTY_BITS: u8 = 24;
const PUZZLE_CONTEXT: &[u8] = b"scotoma-admission-puzzle-v2";

/// Server-issued, socket-bound client puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionChallenge {
    pub nonce: [u8; PUZZLE_NONCE_BYTES],
    pub difficulty_bits: u8,
}

impl AdmissionChallenge {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.difficulty_bits <= MAX_PUZZLE_DIFFICULTY_BITS
    }
}

/// Finds a proof for a server challenge. Intended to run off the async runtime.
#[must_use]
pub fn solve_admission_challenge(
    challenge: AdmissionChallenge,
    tag: &RendezvousTag,
) -> Option<u64> {
    if !challenge.is_valid() {
        return None;
    }
    (0..=u64::MAX).find(|candidate| verify_admission_solution(challenge, tag, *candidate))
}

/// Cheap relay-side verification of one proof candidate.
#[must_use]
pub fn verify_admission_solution(
    challenge: AdmissionChallenge,
    tag: &RendezvousTag,
    solution: u64,
) -> bool {
    if !challenge.is_valid() {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(PUZZLE_CONTEXT);
    hasher.update([challenge.difficulty_bits]);
    hasher.update(challenge.nonce);
    hasher.update(tag.as_bytes());
    hasher.update(solution.to_be_bytes());
    leading_zero_bits(&hasher.finalize()) >= challenge.difficulty_bits
}

fn leading_zero_bits(digest: &[u8]) -> u8 {
    let mut total = 0_u8;
    for byte in digest {
        let zeroes = u8::try_from(byte.leading_zeros()).unwrap_or(8);
        total = total.saturating_add(zeroes);
        if zeroes != 8 {
            break;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use crate::RendezvousTag;

    use super::{
        AdmissionChallenge, MAX_PUZZLE_DIFFICULTY_BITS, solve_admission_challenge,
        verify_admission_solution,
    };

    #[test]
    fn solved_challenge_verifies_and_is_bound_to_tag() {
        let challenge = AdmissionChallenge {
            nonce: [0x42; 16],
            difficulty_bits: 12,
        };
        let tag = RendezvousTag::from_bytes([7; 32]);
        let other_tag = RendezvousTag::from_bytes([8; 32]);
        let solution = solve_admission_challenge(challenge, &tag).expect("solvable puzzle");

        assert!(verify_admission_solution(challenge, &tag, solution));
        assert!(!verify_admission_solution(challenge, &other_tag, solution));
    }

    #[test]
    fn excessive_difficulty_fails_closed() {
        let challenge = AdmissionChallenge {
            nonce: [0; 16],
            difficulty_bits: MAX_PUZZLE_DIFFICULTY_BITS + 1,
        };
        let tag = RendezvousTag::from_bytes([7; 32]);

        assert_eq!(solve_admission_challenge(challenge, &tag), None);
        assert!(!verify_admission_solution(challenge, &tag, 0));
    }
}
