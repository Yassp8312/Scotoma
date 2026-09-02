use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationalEvent {
    UpgradeRateLimited,
    ConnectionCapacityRejected,
    JoinTimedOut,
    JoinRejected,
    ProtocolRejected,
    RoomCapacityRejected,
    ConnectionFrameRateLimited,
    RoomFrameRateLimited,
    RoomExpiredPairing,
    RoomExpiredHandshake,
    RoomExpiredIdle,
    RoomExpiredLifetime,
    RoomBurned,
    RoomDisconnected,
    SlowWriter,
}

impl OperationalEvent {
    const fn name(self) -> &'static str {
        match self {
            Self::UpgradeRateLimited => "upgrade_rate_limited",
            Self::ConnectionCapacityRejected => "connection_capacity_rejected",
            Self::JoinTimedOut => "join_timed_out",
            Self::JoinRejected => "join_rejected",
            Self::ProtocolRejected => "protocol_rejected",
            Self::RoomCapacityRejected => "room_capacity_rejected",
            Self::ConnectionFrameRateLimited => "connection_frame_rate_limited",
            Self::RoomFrameRateLimited => "room_frame_rate_limited",
            Self::RoomExpiredPairing => "room_expired_pairing",
            Self::RoomExpiredHandshake => "room_expired_handshake",
            Self::RoomExpiredIdle => "room_expired_idle",
            Self::RoomExpiredLifetime => "room_expired_lifetime",
            Self::RoomBurned => "room_burned",
            Self::RoomDisconnected => "room_disconnected",
            Self::SlowWriter => "slow_writer",
        }
    }
}

impl Display for OperationalEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{{\"component\":\"scotoma-relay\",\"event\":\"{}\"}}",
            self.name()
        )
    }
}

#[derive(Debug, Default)]
struct Metrics {
    upgrades_accepted_total: AtomicU64,
    active_connections: AtomicU64,
    active_rooms: AtomicU64,
    rooms_created_total: AtomicU64,
    rooms_closed_total: AtomicU64,
    frames_forwarded_total: AtomicU64,
    pending_frames: AtomicU64,
    rejected_joins_total: AtomicU64,
    rate_limit_violations_total: AtomicU64,
    timeouts_total: AtomicU64,
    burns_total: AtomicU64,
    events_dropped_total: AtomicU64,
    protocol_rejections_total: AtomicU64,
    resource_limit_violations_total: AtomicU64,
}

#[derive(Debug, Clone)]
pub(crate) struct Observability {
    metrics: Arc<Metrics>,
    event_sender: Option<SyncSender<OperationalEvent>>,
}

impl Observability {
    pub fn with_stderr_sink(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::default();
        }
        let metrics = Arc::new(Metrics::default());
        let (sender, receiver) = sync_channel(capacity);
        let spawned = std::thread::Builder::new()
            .name("scotoma-relay-events".to_owned())
            .spawn(move || {
                for event in receiver {
                    eprintln!("{event}");
                }
            })
            .is_ok();
        Self {
            metrics,
            event_sender: spawned.then_some(sender),
        }
    }

    pub fn record(&self, event: OperationalEvent) {
        match event {
            OperationalEvent::UpgradeRateLimited
            | OperationalEvent::ConnectionFrameRateLimited
            | OperationalEvent::RoomFrameRateLimited => {
                self.metrics
                    .rate_limit_violations_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::ConnectionCapacityRejected | OperationalEvent::JoinRejected => {
                self.metrics
                    .rejected_joins_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::ProtocolRejected => {
                self.metrics
                    .protocol_rejections_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::RoomCapacityRejected => {
                self.metrics
                    .resource_limit_violations_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::JoinTimedOut
            | OperationalEvent::RoomExpiredPairing
            | OperationalEvent::RoomExpiredHandshake
            | OperationalEvent::RoomExpiredIdle
            | OperationalEvent::RoomExpiredLifetime
            | OperationalEvent::SlowWriter => {
                self.metrics.timeouts_total.fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::RoomBurned => {
                self.metrics.burns_total.fetch_add(1, Ordering::Relaxed);
            }
            OperationalEvent::RoomDisconnected => {}
        }
        if self
            .event_sender
            .as_ref()
            .is_some_and(|sender| sender.try_send(event).is_err())
        {
            self.metrics
                .events_dropped_total
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn connection_opened(&self) {
        self.metrics
            .upgrades_accepted_total
            .fetch_add(1, Ordering::Relaxed);
        self.metrics
            .active_connections
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn connection_closed(&self) {
        saturating_decrement(&self.metrics.active_connections, 1);
    }

    pub fn room_opened(&self) {
        self.metrics
            .rooms_created_total
            .fetch_add(1, Ordering::Relaxed);
        self.metrics.active_rooms.fetch_add(1, Ordering::Relaxed);
    }

    pub fn room_closed(&self, pending_removed: usize) {
        self.metrics
            .rooms_closed_total
            .fetch_add(1, Ordering::Relaxed);
        saturating_decrement(&self.metrics.active_rooms, 1);
        self.pending_removed(pending_removed);
    }

    pub fn frame_forwarded(&self) {
        self.metrics
            .frames_forwarded_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn pending_added(&self) {
        self.metrics.pending_frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pending_removed(&self, count: usize) {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        saturating_decrement(&self.metrics.pending_frames, count);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            upgrades_accepted_total: self.metrics.upgrades_accepted_total.load(Ordering::Relaxed),
            active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
            active_rooms: self.metrics.active_rooms.load(Ordering::Relaxed),
            rooms_created_total: self.metrics.rooms_created_total.load(Ordering::Relaxed),
            rooms_closed_total: self.metrics.rooms_closed_total.load(Ordering::Relaxed),
            frames_forwarded_total: self.metrics.frames_forwarded_total.load(Ordering::Relaxed),
            pending_frames: self.metrics.pending_frames.load(Ordering::Relaxed),
            rejected_joins_total: self.metrics.rejected_joins_total.load(Ordering::Relaxed),
            rate_limit_violations_total: self
                .metrics
                .rate_limit_violations_total
                .load(Ordering::Relaxed),
            timeouts_total: self.metrics.timeouts_total.load(Ordering::Relaxed),
            burns_total: self.metrics.burns_total.load(Ordering::Relaxed),
            events_dropped_total: self.metrics.events_dropped_total.load(Ordering::Relaxed),
            protocol_rejections_total: self
                .metrics
                .protocol_rejections_total
                .load(Ordering::Relaxed),
            resource_limit_violations_total: self
                .metrics
                .resource_limit_violations_total
                .load(Ordering::Relaxed),
        }
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self {
            metrics: Arc::new(Metrics::default()),
            event_sender: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MetricsSnapshot {
    upgrades_accepted_total: u64,
    active_connections: u64,
    active_rooms: u64,
    rooms_created_total: u64,
    rooms_closed_total: u64,
    frames_forwarded_total: u64,
    pending_frames: u64,
    rejected_joins_total: u64,
    rate_limit_violations_total: u64,
    timeouts_total: u64,
    burns_total: u64,
    events_dropped_total: u64,
    protocol_rejections_total: u64,
    resource_limit_violations_total: u64,
}

impl MetricsSnapshot {
    pub fn render(self) -> String {
        format!(
            concat!(
                "scotoma_relay_upgrades_accepted_total {}\n",
                "scotoma_relay_active_connections {}\n",
                "scotoma_relay_active_rooms {}\n",
                "scotoma_relay_rooms_created_total {}\n",
                "scotoma_relay_rooms_closed_total {}\n",
                "scotoma_relay_frames_forwarded_total {}\n",
                "scotoma_relay_pending_frames {}\n",
                "scotoma_relay_rejected_joins_total {}\n",
                "scotoma_relay_rate_limit_violations_total {}\n",
                "scotoma_relay_timeouts_total {}\n",
                "scotoma_relay_burns_total {}\n",
                "scotoma_relay_events_dropped_total {}\n",
                "scotoma_relay_protocol_rejections_total {}\n",
                "scotoma_relay_resource_limit_violations_total {}\n"
            ),
            self.upgrades_accepted_total,
            self.active_connections,
            self.active_rooms,
            self.rooms_created_total,
            self.rooms_closed_total,
            self.frames_forwarded_total,
            self.pending_frames,
            self.rejected_joins_total,
            self.rate_limit_violations_total,
            self.timeouts_total,
            self.burns_total,
            self.events_dropped_total,
            self.protocol_rejections_total,
            self.resource_limit_violations_total,
        )
    }
}

fn saturating_decrement(value: &AtomicU64, amount: u64) {
    let _ = value.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(amount))
    });
}

#[cfg(test)]
mod tests {
    use super::{Observability, OperationalEvent};

    #[test]
    fn event_rendering_has_no_dynamic_fields() {
        let events = [
            OperationalEvent::UpgradeRateLimited,
            OperationalEvent::ConnectionCapacityRejected,
            OperationalEvent::JoinTimedOut,
            OperationalEvent::JoinRejected,
            OperationalEvent::ProtocolRejected,
            OperationalEvent::RoomCapacityRejected,
            OperationalEvent::ConnectionFrameRateLimited,
            OperationalEvent::RoomFrameRateLimited,
            OperationalEvent::RoomExpiredPairing,
            OperationalEvent::RoomExpiredHandshake,
            OperationalEvent::RoomExpiredIdle,
            OperationalEvent::RoomExpiredLifetime,
            OperationalEvent::RoomBurned,
            OperationalEvent::RoomDisconnected,
            OperationalEvent::SlowWriter,
        ];

        for event in events {
            let rendered = event.to_string();
            assert!(rendered.starts_with("{\"component\":\"scotoma-relay\",\"event\":\""));
            assert!(rendered.ends_with("\"}"));
            assert!(!rendered.contains("rendezvous"));
            assert!(!rendered.contains("payload"));
            assert!(!rendered.contains("127.0.0.1"));
        }
    }

    #[test]
    fn metrics_are_aggregate_and_gauges_saturate() {
        let observability = Observability::default();
        observability.connection_closed();
        observability.room_closed(10);
        observability.connection_opened();
        observability.room_opened();
        observability.frame_forwarded();
        observability.pending_added();
        observability.record(OperationalEvent::RoomBurned);
        let rendered = observability.snapshot().render();

        assert!(rendered.contains("scotoma_relay_active_connections 1\n"));
        assert!(rendered.contains("scotoma_relay_active_rooms 1\n"));
        assert!(rendered.contains("scotoma_relay_pending_frames 1\n"));
        assert!(rendered.contains("scotoma_relay_burns_total 1\n"));
        assert!(!rendered.contains('{'));
        assert!(!rendered.contains("tag"));
    }
}
