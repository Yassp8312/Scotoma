use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Observable lifecycle states for one local session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    WaitingForPeer,
    Handshaking,
    Connected,
    Reconnecting,
    Burned,
    Expired,
    Failed,
}

impl SessionState {
    /// Applies one lifecycle event while rejecting ambiguous transitions.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError`] when the event is not valid for the current
    /// state. Terminal sessions reject every event.
    pub fn transition(self, event: SessionEvent) -> Result<Self, TransitionError> {
        let next = match (self, event) {
            (Self::Idle, SessionEvent::StartWaiting) => Self::WaitingForPeer,
            (Self::WaitingForPeer, SessionEvent::PeerAvailable) => Self::Handshaking,
            (Self::Handshaking, SessionEvent::HandshakeSucceeded)
            | (Self::Reconnecting, SessionEvent::Reconnected) => Self::Connected,
            (Self::Handshaking, SessionEvent::HandshakeFailed) => Self::Failed,
            (Self::Connected, SessionEvent::ConnectionLost) => Self::Reconnecting,
            (
                Self::WaitingForPeer | Self::Handshaking | Self::Connected | Self::Reconnecting,
                SessionEvent::Burn,
            ) => Self::Burned,
            (
                Self::WaitingForPeer | Self::Handshaking | Self::Connected | Self::Reconnecting,
                SessionEvent::Expire,
            ) => Self::Expired,
            _ => return Err(TransitionError { state: self, event }),
        };

        Ok(next)
    }

    /// Terminal sessions cannot be resumed or reused.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Burned | Self::Expired | Self::Failed)
    }
}

/// Events that may advance a local session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    StartWaiting,
    PeerAvailable,
    HandshakeSucceeded,
    HandshakeFailed,
    ConnectionLost,
    Reconnected,
    Burn,
    Expire,
}

/// Invalid state-machine transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    pub state: SessionState,
    pub event: SessionEvent,
}

impl Display for TransitionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "event {:?} is invalid while session is {:?}",
            self.event, self.state
        )
    }
}

impl Error for TransitionError {}

#[cfg(test)]
mod tests {
    use super::{SessionEvent, SessionState};

    #[test]
    fn happy_path_reaches_connected() {
        let state = SessionState::Idle
            .transition(SessionEvent::StartWaiting)
            .and_then(|state| state.transition(SessionEvent::PeerAvailable))
            .and_then(|state| state.transition(SessionEvent::HandshakeSucceeded))
            .expect("valid connection path");

        assert_eq!(state, SessionState::Connected);
    }

    #[test]
    fn reconnect_returns_to_connected() {
        let state = SessionState::Connected
            .transition(SessionEvent::ConnectionLost)
            .and_then(|state| state.transition(SessionEvent::Reconnected))
            .expect("valid reconnection path");

        assert_eq!(state, SessionState::Connected);
    }

    #[test]
    fn burn_is_terminal() {
        let burned = SessionState::Connected
            .transition(SessionEvent::Burn)
            .expect("connected sessions may burn");

        assert!(burned.is_terminal());
        assert!(burned.transition(SessionEvent::StartWaiting).is_err());
    }

    #[test]
    fn messages_cannot_skip_the_handshake() {
        assert!(
            SessionState::WaitingForPeer
                .transition(SessionEvent::HandshakeSucceeded)
                .is_err()
        );
    }
}
