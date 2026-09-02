use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use scotoma_protocol::{Frame, FrameKind, RendezvousTag};
use tokio::sync::{Mutex, mpsc};
use zeroize::Zeroizing;

use crate::rate_limit::RateLimiter;
use crate::{PeerRole, RelayConfig, RelayEvent};

type PeerId = u64;

pub(crate) enum Outbound {
    Frame(Vec<u8>),
    Event(RelayEvent),
}

impl std::fmt::Debug for Outbound {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame(frame) => formatter
                .debug_struct("Frame")
                .field("encoded_len", &frame.len())
                .field("encoded", &"[REDACTED]")
                .finish(),
            Self::Event(event) => formatter.debug_tuple("Event").field(event).finish(),
        }
    }
}

#[derive(Debug)]
struct Peer {
    id: PeerId,
    outbound: mpsc::Sender<Outbound>,
}

struct PendingFrame {
    sender_id: PeerId,
    encoded: Zeroizing<Vec<u8>>,
    inserted_at: Instant,
}

impl std::fmt::Debug for PendingFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingFrame")
            .field("sender_id", &self.sender_id)
            .field("encoded_len", &self.encoded.len())
            .field("encoded", &"[REDACTED]")
            .field("inserted_at", &self.inserted_at)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct Room {
    peers: Vec<Peer>,
    pending: VecDeque<PendingFrame>,
    pending_bytes: usize,
    created_at: Instant,
    last_activity: Instant,
    paired_at: Option<Instant>,
    established: bool,
    frame_limiter: RateLimiter,
}

impl Room {
    fn new(now: Instant, config: RelayConfig) -> Self {
        Self {
            peers: Vec::with_capacity(2),
            pending: VecDeque::new(),
            pending_bytes: 0,
            created_at: now,
            last_activity: now,
            paired_at: None,
            established: false,
            frame_limiter: RateLimiter::new(
                config.max_room_frames_per_second,
                config.room_frame_burst,
                now,
            ),
        }
    }

    fn purge_expired_frames(&mut self, now: Instant, config: RelayConfig) {
        while self.pending.front().is_some_and(|pending| {
            now.saturating_duration_since(pending.inserted_at) >= config.pending_frame_ttl
        }) {
            let Some(expired) = self.pending.pop_front() else {
                break;
            };
            self.pending_bytes = self.pending_bytes.saturating_sub(expired.encoded.len());
        }
    }

    fn acknowledge_oldest_from_peer(&mut self, sender_id: PeerId) {
        let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.sender_id != sender_id)
        else {
            return;
        };
        if let Some(acknowledged) = self.pending.remove(index) {
            self.pending_bytes = self
                .pending_bytes
                .saturating_sub(acknowledged.encoded.len());
        }
    }
}

#[derive(Debug)]
struct HubState {
    rooms: HashMap<RendezvousTag, Room>,
}

#[derive(Debug)]
pub(crate) struct Registration {
    pub id: PeerId,
    pub role: PeerRole,
    pub receiver: mpsc::Receiver<Outbound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HubError {
    Capacity,
    RateLimited,
    RoomFull,
    PeerUnavailable,
}

#[derive(Debug, Clone)]
pub(crate) struct RelayHub {
    state: Arc<Mutex<HubState>>,
    next_peer_id: Arc<AtomicU64>,
    config: RelayConfig,
}

impl RelayHub {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(HubState {
                rooms: HashMap::new(),
            })),
            next_peer_id: Arc::new(AtomicU64::new(1)),
            config,
        }
    }

    pub async fn register(&self, tag: RendezvousTag) -> Result<Registration, HubError> {
        let mut state = self.state.lock().await;
        if !state.rooms.contains_key(&tag) && state.rooms.len() >= self.config.max_rooms {
            return Err(HubError::Capacity);
        }

        let id = self.next_peer_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::channel(self.config.outbound_channel_capacity.max(1));
        let now = Instant::now();
        let room = state
            .rooms
            .entry(tag.clone())
            .or_insert_with(|| Room::new(now, self.config));
        room.last_activity = now;

        match room.peers.len() {
            0 => {
                room.peers.push(Peer {
                    id,
                    outbound: sender,
                });
                Ok(Registration {
                    id,
                    role: PeerRole::Initiator,
                    receiver,
                })
            }
            1 => {
                let existing = room.peers.first().ok_or(HubError::PeerUnavailable)?;
                if existing
                    .outbound
                    .try_send(Outbound::Event(RelayEvent::PeerReady(PeerRole::Initiator)))
                    .is_err()
                {
                    state.rooms.remove(&tag);
                    return Err(HubError::PeerUnavailable);
                }

                room.peers.push(Peer {
                    id,
                    outbound: sender,
                });
                room.paired_at = Some(now);
                Ok(Registration {
                    id,
                    role: PeerRole::Responder,
                    receiver,
                })
            }
            _ => Err(HubError::RoomFull),
        }
    }

    pub async fn process_frame(
        &self,
        tag: &RendezvousTag,
        sender_id: PeerId,
        frame: &Frame,
        encoded: Vec<u8>,
    ) -> Result<(), HubError> {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let room = state.rooms.get_mut(tag).ok_or(HubError::PeerUnavailable)?;
        if room.peers.len() != 2 {
            return Err(HubError::PeerUnavailable);
        }
        if !room.peers.iter().any(|peer| peer.id == sender_id) {
            return Err(HubError::PeerUnavailable);
        }
        if !room.frame_limiter.allow(now) {
            return Err(HubError::RateLimited);
        }
        room.purge_expired_frames(now, self.config);
        room.last_activity = now;
        if matches!(
            frame.kind(),
            FrameKind::Ciphertext | FrameKind::Ack | FrameKind::Burn
        ) {
            room.established = true;
        }

        let recipient = room
            .peers
            .iter()
            .find(|peer| peer.id != sender_id)
            .ok_or(HubError::PeerUnavailable)?;
        if frame.kind() == FrameKind::Ciphertext {
            let next_count = room
                .pending
                .len()
                .checked_add(1)
                .ok_or(HubError::Capacity)?;
            let next_bytes = room
                .pending_bytes
                .checked_add(encoded.len())
                .ok_or(HubError::Capacity)?;
            if next_count > self.config.protocol_limits.max_pending_frames_per_room
                || next_bytes > self.config.protocol_limits.max_pending_bytes_per_room
            {
                return Err(HubError::Capacity);
            }
        }

        let forward_result = recipient
            .outbound
            .try_send(Outbound::Frame(encoded.clone()))
            .map_err(|_| HubError::PeerUnavailable);

        if frame.kind() == FrameKind::Burn {
            state.rooms.remove(tag);
            return forward_result;
        }
        forward_result?;

        match frame.kind() {
            FrameKind::Ciphertext => {
                room.pending_bytes += encoded.len();
                room.pending.push_back(PendingFrame {
                    sender_id,
                    encoded: Zeroizing::new(encoded),
                    inserted_at: now,
                });
            }
            FrameKind::Ack => room.acknowledge_oldest_from_peer(sender_id),
            FrameKind::Handshake | FrameKind::Burn | FrameKind::Error => {}
        }
        Ok(())
    }

    pub async fn expire(&self, tag: &RendezvousTag) -> bool {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let Some(room) = state.rooms.get_mut(tag) else {
            return true;
        };
        room.purge_expired_frames(now, self.config);
        let waiting_expired = room.peers.len() < 2
            && now.saturating_duration_since(room.created_at) >= self.config.pairing_timeout;
        let handshake_expired = room.peers.len() == 2
            && !room.established
            && room.paired_at.is_some_and(|paired_at| {
                now.saturating_duration_since(paired_at) >= self.config.session_handshake_timeout
            });
        let idle_expired =
            now.saturating_duration_since(room.last_activity) >= self.config.room_idle_ttl;
        let lifetime_expired =
            now.saturating_duration_since(room.created_at) >= self.config.max_room_lifetime;
        if !(waiting_expired || handshake_expired || idle_expired || lifetime_expired) {
            return false;
        }

        let Some(room) = state.rooms.remove(tag) else {
            return true;
        };
        for peer in room.peers {
            let _ = peer
                .outbound
                .try_send(Outbound::Event(RelayEvent::RoomClosed));
        }
        true
    }

    pub async fn unregister(&self, tag: &RendezvousTag, peer_id: PeerId) {
        let mut state = self.state.lock().await;
        let Some(room) = state.rooms.remove(tag) else {
            return;
        };

        for peer in room.peers {
            if peer.id != peer_id {
                let _ = peer
                    .outbound
                    .try_send(Outbound::Event(RelayEvent::RoomClosed));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use scotoma_protocol::{Frame, FrameKind, ProtocolLimits, RENDEZVOUS_TAG_BYTES, RendezvousTag};
    use tokio::time::sleep;

    use super::{HubError, Outbound, RelayHub};
    use crate::{PeerRole, RelayConfig, RelayEvent};

    fn tag(value: u8) -> RendezvousTag {
        RendezvousTag::from_bytes([value; RENDEZVOUS_TAG_BYTES])
    }

    fn config() -> RelayConfig {
        RelayConfig {
            max_rooms: 10,
            outbound_channel_capacity: 4,
            ..RelayConfig::default()
        }
    }

    fn encoded_frame(kind: FrameKind, counter: u64) -> (Frame, Vec<u8>) {
        let frame = Frame::new(
            kind,
            counter,
            vec![u8::try_from(counter).unwrap_or(0); 16],
            ProtocolLimits::default(),
        )
        .expect("bounded frame");
        let encoded = frame.encode().expect("encodable frame");
        (frame, encoded)
    }

    async fn pending_stats(hub: &RelayHub, room_tag: &RendezvousTag) -> (usize, usize) {
        let state = hub.state.lock().await;
        let room = state.rooms.get(room_tag).expect("room exists");
        (room.pending.len(), room.pending_bytes)
    }

    #[tokio::test]
    async fn room_accepts_exactly_two_peers() {
        let hub = RelayHub::new(config());
        let room_tag = tag(1);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");

        assert_eq!(first.role, PeerRole::Initiator);
        assert_eq!(second.role, PeerRole::Responder);
        assert!(matches!(
            first_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::PeerReady(PeerRole::Initiator)))
        ));
        assert!(matches!(
            hub.register(room_tag).await,
            Err(HubError::RoomFull)
        ));
    }

    #[tokio::test]
    async fn disconnect_destroys_room_and_notifies_peer() {
        let hub = RelayHub::new(config());
        let room_tag = tag(2);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;

        hub.unregister(&room_tag, first.id).await;

        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
        let replacement = hub.register(room_tag).await.expect("new room");
        assert_eq!(replacement.role, PeerRole::Initiator);
    }

    #[tokio::test]
    async fn room_capacity_is_bounded() {
        let hub = RelayHub::new(RelayConfig {
            max_rooms: 1,
            ..config()
        });
        hub.register(tag(3)).await.expect("first room");

        assert!(matches!(
            hub.register(tag(4)).await,
            Err(HubError::Capacity)
        ));
    }

    #[tokio::test]
    async fn ciphertext_is_retained_until_the_opposite_peer_acknowledges() {
        let hub = RelayHub::new(config());
        let room_tag = tag(5);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let _ = first_receiver.recv().await;
        let (ciphertext, encoded_ciphertext) = encoded_frame(FrameKind::Ciphertext, 0);

        hub.process_frame(&room_tag, first.id, &ciphertext, encoded_ciphertext.clone())
            .await
            .expect("ciphertext forwarded");
        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Frame(frame)) if frame == encoded_ciphertext
        ));
        assert_eq!(pending_stats(&hub, &room_tag).await.0, 1);

        let (ack, encoded_ack) = encoded_frame(FrameKind::Ack, 0);
        hub.process_frame(&room_tag, second.id, &ack, encoded_ack)
            .await
            .expect("ACK forwarded");

        assert_eq!(pending_stats(&hub, &room_tag).await, (0, 0));
    }

    #[tokio::test]
    async fn pending_memory_limits_fail_closed() {
        let limits = ProtocolLimits {
            max_pending_frames_per_room: 1,
            ..ProtocolLimits::default()
        };
        let hub = RelayHub::new(RelayConfig {
            protocol_limits: limits,
            ..config()
        });
        let room_tag = tag(6);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let _ = first_receiver.recv().await;
        let (first_frame, first_encoded) = encoded_frame(FrameKind::Ciphertext, 0);
        hub.process_frame(&room_tag, first.id, &first_frame, first_encoded)
            .await
            .expect("first pending frame");
        let _ = second_receiver.recv().await;
        let (second_frame, second_encoded) = encoded_frame(FrameKind::Ciphertext, 1);

        let error = hub
            .process_frame(&room_tag, first.id, &second_frame, second_encoded)
            .await
            .expect_err("second pending frame exceeds the limit");

        assert_eq!(error, HubError::Capacity);
        assert_eq!(pending_stats(&hub, &room_tag).await.0, 1);
    }

    #[tokio::test]
    async fn aggregate_room_frame_burst_is_bounded() {
        let hub = RelayHub::new(RelayConfig {
            max_room_frames_per_second: 1,
            room_frame_burst: 1,
            ..config()
        });
        let room_tag = tag(10);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let _second = hub.register(room_tag.clone()).await.expect("second peer");
        let _ = first_receiver.recv().await;
        let (allowed, allowed_encoded) = encoded_frame(FrameKind::Handshake, 0);
        hub.process_frame(&room_tag, first.id, &allowed, allowed_encoded)
            .await
            .expect("initial burst token");
        let (limited, limited_encoded) = encoded_frame(FrameKind::Handshake, 1);

        let error = hub
            .process_frame(&room_tag, first.id, &limited, limited_encoded)
            .await
            .expect_err("room burst must be bounded");

        assert_eq!(error, HubError::RateLimited);
    }

    #[tokio::test]
    async fn pending_ciphertext_expires_without_expiring_an_active_room() {
        let hub = RelayHub::new(RelayConfig {
            pending_frame_ttl: Duration::from_millis(10),
            room_idle_ttl: Duration::from_secs(1),
            ..config()
        });
        let room_tag = tag(7);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let _ = first_receiver.recv().await;
        let (ciphertext, encoded) = encoded_frame(FrameKind::Ciphertext, 0);
        hub.process_frame(&room_tag, first.id, &ciphertext, encoded)
            .await
            .expect("pending ciphertext");
        drop(second);

        sleep(Duration::from_millis(25)).await;

        assert!(!hub.expire(&room_tag).await);
        assert_eq!(pending_stats(&hub, &room_tag).await, (0, 0));
    }

    #[tokio::test]
    async fn idle_room_expiration_notifies_both_peers_and_purges_state() {
        let hub = RelayHub::new(RelayConfig {
            pending_frame_ttl: Duration::from_secs(1),
            room_idle_ttl: Duration::from_millis(10),
            ..config()
        });
        let room_tag = tag(8);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let _ = first_receiver.recv().await;

        sleep(Duration::from_millis(25)).await;

        assert!(hub.expire(&room_tag).await);
        assert!(matches!(
            first_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
        assert!(!hub.state.lock().await.rooms.contains_key(&room_tag));
    }

    #[tokio::test]
    async fn pairing_timeout_releases_a_room_with_one_peer() {
        let hub = RelayHub::new(RelayConfig {
            pairing_timeout: Duration::from_millis(10),
            room_idle_ttl: Duration::from_secs(1),
            ..config()
        });
        let room_tag = tag(11);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;

        sleep(Duration::from_millis(25)).await;

        assert!(hub.expire(&room_tag).await);
        assert!(matches!(
            first_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
    }

    #[tokio::test]
    async fn incomplete_session_handshake_expires_absolutely() {
        let hub = RelayHub::new(RelayConfig {
            session_handshake_timeout: Duration::from_millis(10),
            room_idle_ttl: Duration::from_secs(1),
            ..config()
        });
        let room_tag = tag(12);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let _ = first_receiver.recv().await;

        sleep(Duration::from_millis(25)).await;

        assert!(hub.expire(&room_tag).await);
        assert!(matches!(
            first_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
    }

    #[tokio::test]
    async fn absolute_room_lifetime_cannot_be_extended_by_activity() {
        let hub = RelayHub::new(RelayConfig {
            max_room_lifetime: Duration::from_millis(10),
            room_idle_ttl: Duration::from_secs(1),
            ..config()
        });
        let room_tag = tag(13);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let mut first_receiver = first.receiver;
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let _ = first_receiver.recv().await;
        let (ciphertext, encoded) = encoded_frame(FrameKind::Ciphertext, 0);
        hub.process_frame(&room_tag, first.id, &ciphertext, encoded)
            .await
            .expect("room marked established");
        let _ = second_receiver.recv().await;

        sleep(Duration::from_millis(25)).await;

        assert!(hub.expire(&room_tag).await);
        assert!(matches!(
            first_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Event(RelayEvent::RoomClosed))
        ));
    }

    #[tokio::test]
    async fn burn_is_forwarded_once_and_purges_the_room() {
        let hub = RelayHub::new(config());
        let room_tag = tag(9);
        let first = hub.register(room_tag.clone()).await.expect("first peer");
        let second = hub.register(room_tag.clone()).await.expect("second peer");
        let mut second_receiver = second.receiver;
        let (burn, encoded_burn) = encoded_frame(FrameKind::Burn, 0);

        hub.process_frame(&room_tag, first.id, &burn, encoded_burn.clone())
            .await
            .expect("burn forwarded");

        assert!(matches!(
            second_receiver.recv().await,
            Some(Outbound::Frame(frame)) if frame == encoded_burn
        ));
        assert!(!hub.state.lock().await.rooms.contains_key(&room_tag));
    }
}
