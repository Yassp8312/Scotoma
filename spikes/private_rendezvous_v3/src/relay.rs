//! In-memory opaque relay model for the isolated v3 research spike.
//!
//! The relay sees only epoch, rendezvous tag, connection identifiers and opaque
//! bounded frames. It is not a network server and must not enter production.

use std::collections::{BTreeMap, VecDeque};

pub const MAX_RELAY_FRAME_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RendezvousKey {
    pub epoch: u64,
    pub tag: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelayEvent {
    Paired { peer_id: u64 },
    Frame(Vec<u8>),
    Collision,
    Burn,
    Timeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistrationOutcome {
    Waiting,
    Paired { peer_id: u64 },
    Collision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelayError {
    DuplicateParticipant,
    UnknownParticipant,
    NotPaired,
    FrameTooLarge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Slot {
    Waiting(u64),
    Paired(u64, u64),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryRelay {
    rooms: BTreeMap<RendezvousKey, Slot>,
    participants: BTreeMap<u64, RendezvousKey>,
    events: BTreeMap<u64, VecDeque<RelayEvent>>,
}

impl MemoryRelay {
    pub fn register(
        &mut self,
        key: RendezvousKey,
        participant_id: u64,
    ) -> Result<RegistrationOutcome, RelayError> {
        if self.participants.contains_key(&participant_id) {
            return Err(RelayError::DuplicateParticipant);
        }

        match self.rooms.get(&key).copied() {
            None => {
                self.rooms.insert(key, Slot::Waiting(participant_id));
                self.participants.insert(participant_id, key);
                self.events.entry(participant_id).or_default();
                Ok(RegistrationOutcome::Waiting)
            }
            Some(Slot::Waiting(first)) => {
                self.rooms.insert(key, Slot::Paired(first, participant_id));
                self.participants.insert(participant_id, key);
                self.events
                    .entry(first)
                    .or_default()
                    .push_back(RelayEvent::Paired {
                        peer_id: participant_id,
                    });
                self.events
                    .entry(participant_id)
                    .or_default()
                    .push_back(RelayEvent::Paired { peer_id: first });
                Ok(RegistrationOutcome::Paired { peer_id: first })
            }
            Some(Slot::Paired(first, second)) => {
                self.rooms.remove(&key);
                self.participants.remove(&first);
                self.participants.remove(&second);
                for id in [first, second, participant_id] {
                    self.replace_events(id, RelayEvent::Collision);
                }
                Ok(RegistrationOutcome::Collision)
            }
        }
    }

    pub fn send(&mut self, sender_id: u64, frame: Vec<u8>) -> Result<(), RelayError> {
        if frame.len() > MAX_RELAY_FRAME_BYTES {
            return Err(RelayError::FrameTooLarge);
        }
        let key = *self
            .participants
            .get(&sender_id)
            .ok_or(RelayError::UnknownParticipant)?;
        let peer = match self.rooms.get(&key) {
            Some(Slot::Paired(first, second)) if *first == sender_id => *second,
            Some(Slot::Paired(first, second)) if *second == sender_id => *first,
            _ => return Err(RelayError::NotPaired),
        };
        self.events
            .entry(peer)
            .or_default()
            .push_back(RelayEvent::Frame(frame));
        Ok(())
    }

    pub fn burn(&mut self, sender_id: u64) -> Result<(), RelayError> {
        let key = *self
            .participants
            .get(&sender_id)
            .ok_or(RelayError::UnknownParticipant)?;
        let (first, second) = match self.rooms.remove(&key) {
            Some(Slot::Paired(first, second)) => (first, second),
            Some(waiting @ Slot::Waiting(_)) => {
                self.rooms.insert(key, waiting);
                return Err(RelayError::NotPaired);
            }
            None => return Err(RelayError::UnknownParticipant),
        };
        let peer = if first == sender_id { second } else { first };
        self.participants.remove(&first);
        self.participants.remove(&second);
        self.events.remove(&sender_id);
        self.replace_events(peer, RelayEvent::Burn);
        Ok(())
    }

    pub fn timeout(&mut self, key: RendezvousKey) {
        if let Some(slot) = self.rooms.remove(&key) {
            match slot {
                Slot::Waiting(participant) => {
                    self.participants.remove(&participant);
                    self.replace_events(participant, RelayEvent::Timeout);
                }
                Slot::Paired(first, second) => {
                    self.participants.remove(&first);
                    self.participants.remove(&second);
                    self.replace_events(first, RelayEvent::Timeout);
                    self.replace_events(second, RelayEvent::Timeout);
                }
            }
        }
    }

    pub fn poll_event(&mut self, participant_id: u64) -> Option<RelayEvent> {
        let queue = self.events.get_mut(&participant_id)?;
        let event = queue.pop_front();
        if queue.is_empty() && !self.participants.contains_key(&participant_id) {
            self.events.remove(&participant_id);
        }
        event
    }

    #[must_use]
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    #[must_use]
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    fn replace_events(&mut self, participant_id: u64, event: RelayEvent) {
        self.events.insert(participant_id, VecDeque::from([event]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: u8) -> RendezvousKey {
        RendezvousKey {
            epoch: 7,
            tag: [value; 32],
        }
    }

    fn pair(relay: &mut MemoryRelay, rendezvous: RendezvousKey) {
        assert_eq!(
            relay.register(rendezvous, 11),
            Ok(RegistrationOutcome::Waiting)
        );
        assert_eq!(
            relay.register(rendezvous, 22),
            Ok(RegistrationOutcome::Paired { peer_id: 11 })
        );
        assert_eq!(
            relay.poll_event(11),
            Some(RelayEvent::Paired { peer_id: 22 })
        );
        assert_eq!(
            relay.poll_event(22),
            Some(RelayEvent::Paired { peer_id: 11 })
        );
    }

    #[test]
    fn pair_routes_opaque_frames_without_interpretation() {
        let mut relay = MemoryRelay::default();
        pair(&mut relay, key(1));
        let opaque = vec![0, 255, 7, 9];
        relay.send(11, opaque.clone()).unwrap();
        assert_eq!(relay.poll_event(22), Some(RelayEvent::Frame(opaque)));
        assert_eq!(relay.room_count(), 1);
        assert_eq!(relay.participant_count(), 2);
    }

    #[test]
    fn different_tags_never_pair() {
        let mut relay = MemoryRelay::default();
        assert_eq!(relay.register(key(1), 11), Ok(RegistrationOutcome::Waiting));
        assert_eq!(relay.register(key(2), 22), Ok(RegistrationOutcome::Waiting));
        assert_eq!(relay.room_count(), 2);
    }

    #[test]
    fn third_participant_invalidates_the_entire_pair() {
        let mut relay = MemoryRelay::default();
        let rendezvous = key(1);
        pair(&mut relay, rendezvous);
        relay.send(11, vec![1, 2, 3]).unwrap();
        assert_eq!(
            relay.register(rendezvous, 33),
            Ok(RegistrationOutcome::Collision)
        );
        for participant in [11, 22, 33] {
            assert_eq!(relay.poll_event(participant), Some(RelayEvent::Collision));
            assert_eq!(relay.poll_event(participant), None);
        }
        assert_eq!(relay.room_count(), 0);
        assert_eq!(relay.participant_count(), 0);
    }

    #[test]
    fn oversized_frame_is_rejected_without_delivery() {
        let mut relay = MemoryRelay::default();
        pair(&mut relay, key(1));
        assert_eq!(
            relay.send(11, vec![0; MAX_RELAY_FRAME_BYTES + 1]),
            Err(RelayError::FrameTooLarge)
        );
        assert_eq!(relay.poll_event(22), None);
    }

    #[test]
    fn burn_discards_queued_frames_and_notifies_the_peer() {
        let mut relay = MemoryRelay::default();
        pair(&mut relay, key(1));
        relay.send(11, vec![1, 2, 3]).unwrap();
        relay.burn(11).unwrap();
        assert_eq!(relay.poll_event(22), Some(RelayEvent::Burn));
        assert_eq!(relay.poll_event(22), None);
        assert_eq!(relay.room_count(), 0);
        assert_eq!(relay.participant_count(), 0);
    }

    #[test]
    fn timeout_purges_waiting_and_paired_rooms() {
        let mut relay = MemoryRelay::default();
        let waiting = key(1);
        relay.register(waiting, 11).unwrap();
        relay.timeout(waiting);
        assert_eq!(relay.poll_event(11), Some(RelayEvent::Timeout));

        let paired = key(2);
        pair(&mut relay, paired);
        relay.timeout(paired);
        assert_eq!(relay.poll_event(11), Some(RelayEvent::Timeout));
        assert_eq!(relay.poll_event(22), Some(RelayEvent::Timeout));
        assert_eq!(relay.room_count(), 0);
    }

    #[test]
    fn waiting_or_unknown_participant_cannot_send() {
        let mut relay = MemoryRelay::default();
        relay.register(key(1), 11).unwrap();
        assert_eq!(relay.send(11, vec![1]), Err(RelayError::NotPaired));
        assert_eq!(relay.send(99, vec![1]), Err(RelayError::UnknownParticipant));
    }
}
