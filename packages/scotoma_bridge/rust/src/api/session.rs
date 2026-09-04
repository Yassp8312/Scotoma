//! Production session boundary. Credentials enter through a bounded manual ABI;
//! generated FRB calls receive only opaque one-shot tokens and session handles.

use std::collections::VecDeque;
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

#[cfg(target_arch = "wasm32")]
use crate::web_transport::{
    WebCommand, WebError, WebEvent, WebExit, WebProgress, authenticate_derived as authenticate_web,
};
use flutter_rust_bridge::frb;
#[cfg(not(target_arch = "wasm32"))]
use scotoma_client::{
    ClientError, ClientProgress, DerivedClientConfig, LiveCommand, LiveEvent, LiveExit,
    authenticate_derived,
};
use scotoma_protocol::{
    MAX_NUMERIC_DIGITS, NumericPassword, PasswordKdfParams, RoomCode, SessionSecret,
    derive_session_secret,
};
#[cfg(test)]
use zeroize::Zeroize;
use zeroize::Zeroizing;

pub const MAX_MESSAGE_BYTES: usize = 16_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialPhase {
    Empty,
    Staged,
    Deriving,
    Ready,
    Connecting,
    WaitingForPeer,
    Handshaking,
    Connected,
    Burning,
    PeerBurned,
    ConnectionLost,
    DeliveryTimedOut,
    Closed,
    Failed,
}

struct Inputs {
    room: RoomCode,
    password: NumericPassword,
}

struct StagedInput {
    token: u32,
    inputs: Inputs,
}

static NEXT_STAGE_TOKEN: AtomicU32 = AtomicU32::new(1);
static DERIVATION_RUNNING: AtomicBool = AtomicBool::new(false);
static STAGED_INPUT: Mutex<Option<StagedInput>> = Mutex::new(None);
static STAGED_MESSAGE: Mutex<Option<StagedMessage>> = Mutex::new(None);

struct StagedMessage {
    token: u32,
    bytes: Zeroizing<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatEventKind {
    Delivered,
    Received,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChatEvent {
    pub kind: ChatEventKind,
    pub text: String,
}

struct State {
    phase: CredentialPhase,
    inputs: Option<Inputs>,
    material: Option<ReadyMaterial>,
    verification_code: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    commands: Option<tokio::sync::mpsc::Sender<LiveCommand>>,
    #[cfg(not(target_arch = "wasm32"))]
    events: VecDeque<LiveEvent>,
    #[cfg(target_arch = "wasm32")]
    commands: VecDeque<WebCommand>,
    #[cfg(target_arch = "wasm32")]
    events: VecDeque<WebEvent>,
}

struct ReadyMaterial {
    room: RoomCode,
    secret: SessionSecret,
}

/// One-shot production session. It cannot be restaged or reopened.
#[frb(opaque)]
pub struct NativeSession {
    state: Mutex<State>,
}

impl NativeSession {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: CredentialPhase::Empty,
                inputs: None,
                material: None,
                verification_code: None,
                #[cfg(not(target_arch = "wasm32"))]
                commands: None,
                #[cfg(not(target_arch = "wasm32"))]
                events: VecDeque::new(),
                #[cfg(target_arch = "wasm32")]
                commands: VecDeque::new(),
                #[cfg(target_arch = "wasm32")]
                events: VecDeque::new(),
            }),
        }
    }

    /// Claims credentials already staged through the bounded, platform-specific
    /// entrypoint. Only this non-secret token crosses generated FRB serialization.
    #[frb(sync)]
    pub fn claim(&self, token: u32) -> bool {
        let mut state = self.lock_state();
        if state.phase != CredentialPhase::Empty {
            return false;
        }
        let Some(inputs) = take_staged(token) else {
            state.phase = CredentialPhase::Failed;
            return false;
        };
        state.inputs = Some(inputs);
        state.phase = CredentialPhase::Staged;
        true
    }

    #[cfg(test)]
    fn stage_and_wipe(&self, room: &mut [u8], password: &mut [u8]) -> bool {
        // Borrowed guards also allow safe inspection of live bytes in tests.
        // Decoded Vecs have initialized input bytes only in their live slices.
        let room = InputWipe(room);
        let password = InputWipe(password);
        match crate::panic_boundary::contain(|| self.accept(room.0, password.0)) {
            Ok(accepted) => accepted,
            Err(()) => {
                let mut state = self.lock_state();
                if state.phase != CredentialPhase::Closed {
                    state.inputs = None;
                    state.material = None;
                    state.phase = CredentialPhase::Failed;
                }
                false
            }
        }
    }

    #[cfg(test)]
    fn accept(&self, room: &[u8], password: &[u8]) -> bool {
        let mut state = self.lock_state();
        // Repeated calls cannot replace the staged pair or reopen a session.
        if state.phase != CredentialPhase::Empty {
            return false;
        }
        let parsed = (|| {
            let room = RoomCode::parse(numeric_text(room)?).map_err(|_| ())?;
            let password = NumericPassword::parse(numeric_text(password)?).map_err(|_| ())?;
            Ok::<_, ()>(Inputs { room, password })
        })();
        match parsed {
            Ok(inputs) => {
                state.inputs = Some(inputs);
                state.phase = CredentialPhase::Staged;
                true
            }
            Err(()) => {
                state.phase = CredentialPhase::Failed;
                false
            }
        }
    }

    /// Expensive work runs on the async FRB worker, never in stage(). Only
    /// the opaque handle crosses this call; no numeric bytes remain in Dart args.
    pub fn derive(&self) -> CredentialPhase {
        self.derive_with(|inputs| {
            let _permit = DerivationGuard::acquire()?;
            derive_session_secret(&inputs.room, &inputs.password, PasswordKdfParams::default())
                .map_err(|_| ())
        })
    }

    fn derive_with(
        &self,
        work: impl FnOnce(&Inputs) -> Result<SessionSecret, ()>,
    ) -> CredentialPhase {
        let inputs = {
            let mut state = self.lock_state();
            if state.phase == CredentialPhase::Empty {
                state.phase = CredentialPhase::Failed;
            }
            if state.phase != CredentialPhase::Staged {
                return state.phase;
            }
            let Some(inputs) = state.inputs.take() else {
                state.phase = CredentialPhase::Failed;
                return state.phase;
            };
            state.phase = CredentialPhase::Deriving;
            inputs
        };
        let result = crate::panic_boundary::contain(|| work(&inputs)).unwrap_or(Err(()));
        let room = inputs.room.clone();
        // The password is destroyed immediately after local derivation.
        drop(inputs);
        let mut state = self.lock_state();
        if state.phase != CredentialPhase::Deriving {
            return state.phase; // late secret result is dropped, never installed
        }
        match result {
            Ok(secret) => {
                state.material = Some(ReadyMaterial { room, secret });
                state.phase = CredentialPhase::Ready;
            }
            Err(()) => state.phase = CredentialPhase::Failed,
        }
        state.phase
    }

    #[frb(sync)]
    pub fn status(&self) -> CredentialPhase {
        self.lock_state().phase
    }

    /// Short channel binding derived from the confirmed Noise transcript.
    /// It is intentionally unavailable before authentication and erased with
    /// every terminal session transition.
    #[frb(sync)]
    pub fn verification_code(&self) -> Option<String> {
        self.lock_state().verification_code.clone()
    }

    /// Claims a message staged through the bounded platform entrypoint and
    /// queues it for authenticated encryption. Only the token crosses FRB.
    #[frb(sync)]
    pub fn claim_message(&self, token: u32) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Connected || state.commands.len() >= 2 {
                let _ = take_staged_message(token);
                return false;
            }
            let Some(message) = take_staged_message(token) else {
                return false;
            };
            state.commands.push_back(WebCommand::SendText(message));
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let sender = {
                let state = self.lock_state();
                if state.phase != CredentialPhase::Connected {
                    let _ = take_staged_message(token);
                    return false;
                }
                state.commands.clone()
            };
            let Some(message) = take_staged_message(token) else {
                return false;
            };
            sender.is_some_and(|sender| sender.try_send(LiveCommand::SendText(message)).is_ok())
        }
    }

    #[frb(sync)]
    pub fn take_event(&self) -> Option<ChatEvent> {
        #[cfg(target_arch = "wasm32")]
        {
            let event = self.lock_state().events.pop_front()?;
            return Some(match event {
                WebEvent::Delivered => ChatEvent {
                    kind: ChatEventKind::Delivered,
                    text: String::new(),
                },
                WebEvent::ReceivedText(mut text) => ChatEvent {
                    kind: ChatEventKind::Received,
                    text: std::mem::take(&mut *text),
                },
                WebEvent::PeerBurned => return None,
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let event = self.lock_state().events.pop_front()?;
            Some(match event {
                LiveEvent::Delivered(_) => ChatEvent {
                    kind: ChatEventKind::Delivered,
                    text: String::new(),
                },
                LiveEvent::ReceivedText(mut text) => ChatEvent {
                    kind: ChatEventKind::Received,
                    text: std::mem::take(&mut *text),
                },
                LiveEvent::PeerBurned => return None,
            })
        }
    }

    /// Requests an authenticated terminal BURN. Closing due to backgrounding
    /// uses `close` instead and never pretends that BURN reached the peer.
    #[frb(sync)]
    pub fn burn(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Connected || state.commands.len() >= 2 {
                return false;
            }
            state.commands.push_back(WebCommand::Burn);
            state.phase = CredentialPhase::Burning;
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Connected {
                return false;
            }
            let Some(sender) = state.commands.clone() else {
                return false;
            };
            if sender.try_send(LiveCommand::Burn).is_err() {
                return false;
            }
            state.phase = CredentialPhase::Burning;
            true
        }
    }

    /// Owns the live socket until cancellation or a terminal transport event.
    /// Only a bounded relay URL and coarse phase values cross the bridge.
    pub async fn run_transport(&self, relay_url: String) -> CredentialPhase {
        #[cfg(target_arch = "wasm32")]
        return self.run_web_transport(relay_url).await;

        #[cfg(not(target_arch = "wasm32"))]
        self.run_native_transport(relay_url).await
    }

    #[cfg(target_arch = "wasm32")]
    async fn run_web_transport(&self, relay_url: String) -> CredentialPhase {
        if !valid_web_relay_url(&relay_url) {
            return self.fail_web_transport(WebError::Authentication);
        }
        let material = {
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Ready {
                return state.phase;
            }
            let Some(material) = state.material.take() else {
                state.phase = CredentialPhase::Failed;
                return state.phase;
            };
            state.phase = CredentialPhase::Connecting;
            material
        };
        let client = authenticate_web(
            &relay_url,
            material.room,
            material.secret,
            std::time::Duration::from_secs(30),
            |progress| self.set_web_progress(progress),
            || self.status() == CredentialPhase::Closed,
        )
        .await;
        let client = match client {
            Ok(client) => client,
            Err(WebError::Cancelled) => return CredentialPhase::Closed,
            Err(error) => return self.fail_web_transport(error),
        };
        let Some(verification_code) = client.verification_code() else {
            return self.fail_web_transport(WebError::Authentication);
        };
        {
            let mut state = self.lock_state();
            if state.phase == CredentialPhase::Closed {
                return CredentialPhase::Closed;
            }
            state.verification_code = Some(verification_code);
            state.phase = CredentialPhase::Connected;
        }
        let result = client
            .run_live(
                || self.lock_state().commands.pop_front(),
                |event| self.push_web_event(event),
                || self.status() == CredentialPhase::Closed,
            )
            .await;
        let mut state = self.lock_state();
        state.commands.clear();
        state.verification_code = None;
        if state.phase == CredentialPhase::Closed {
            return state.phase;
        }
        match result {
            Ok(WebExit::LocalClose | WebExit::LocalBurn) => {
                state.phase = CredentialPhase::Closed;
            }
            Ok(WebExit::PeerBurn) => {
                state.phase = CredentialPhase::PeerBurned;
            }
            Err(error) => {
                state.events.clear();
                state.phase = phase_for_web_error(error);
            }
        }
        state.phase
    }

    #[cfg(target_arch = "wasm32")]
    fn set_web_progress(&self, progress: WebProgress) {
        let mut state = self.lock_state();
        if state.phase == CredentialPhase::Closed {
            return;
        }
        state.phase = match progress {
            WebProgress::Connecting => CredentialPhase::Connecting,
            WebProgress::WaitingForPeer => CredentialPhase::WaitingForPeer,
            WebProgress::Handshaking | WebProgress::Connected => CredentialPhase::Handshaking,
        };
    }

    #[cfg(target_arch = "wasm32")]
    fn push_web_event(&self, event: WebEvent) {
        let mut state = self.lock_state();
        if state.phase == CredentialPhase::Closed {
            return;
        }
        if matches!(event, WebEvent::PeerBurned) {
            state.phase = CredentialPhase::PeerBurned;
        } else if state.events.len() < 100 {
            state.events.push_back(event);
        } else {
            state.events.clear();
            state.phase = CredentialPhase::Failed;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn fail_web_transport(&self, error: WebError) -> CredentialPhase {
        let mut state = self.lock_state();
        if state.phase != CredentialPhase::Closed {
            state.material = None;
            state.verification_code = None;
            state.commands.clear();
            state.events.clear();
            state.phase = phase_for_web_error(error);
        }
        state.phase
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn run_native_transport(&self, relay_url: String) -> CredentialPhase {
        if !valid_relay_url(&relay_url) {
            return self.fail_transport();
        }
        let material = {
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Ready {
                return state.phase;
            }
            let Some(material) = state.material.take() else {
                state.phase = CredentialPhase::Failed;
                return state.phase;
            };
            state.phase = CredentialPhase::Connecting;
            material
        };
        let connect = authenticate_derived(
            DerivedClientConfig {
                relay_url,
                room: material.room,
                secret: material.secret,
                operation_timeout: std::time::Duration::from_secs(30),
            },
            |progress| self.set_progress(progress),
        );
        tokio::pin!(connect);
        let client = tokio::select! {
            result = &mut connect => match result {
                Ok(client) => client,
                Err(error) => return self.fail_with_client_error(error),
            },
            () = self.wait_for_close() => return CredentialPhase::Closed,
        };
        let Some(verification_code) = client.handshake_hash().map(format_verification_code) else {
            return self.fail_transport();
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(2);
        {
            let mut state = self.lock_state();
            if state.phase == CredentialPhase::Closed {
                return CredentialPhase::Closed;
            }
            state.commands = Some(sender);
            state.verification_code = Some(verification_code);
            state.phase = CredentialPhase::Connected;
        }
        let result = client
            .run_live(
                receiver,
                |event| self.push_event(event),
                || self.status() == CredentialPhase::Closed,
            )
            .await;
        let mut state = self.lock_state();
        state.commands = None;
        state.verification_code = None;
        if state.phase == CredentialPhase::Closed {
            return state.phase;
        }
        match result {
            Ok(LiveExit::LocalClose | LiveExit::LocalBurn) => {
                state.phase = CredentialPhase::Closed;
            }
            Ok(LiveExit::PeerBurn) => {
                state.phase = CredentialPhase::PeerBurned;
            }
            Err(error) => {
                state.events.clear();
                state.phase = phase_for_client_error(error);
            }
        }
        state.phase
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn wait_for_close(&self) {
        while self.status() != CredentialPhase::Closed {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_progress(&self, progress: ClientProgress) {
        let mut state = self.lock_state();
        if state.phase == CredentialPhase::Closed {
            return;
        }
        state.phase = match progress {
            ClientProgress::Connecting => CredentialPhase::Connecting,
            ClientProgress::WaitingForPeer => CredentialPhase::WaitingForPeer,
            ClientProgress::Handshaking => CredentialPhase::Handshaking,
            ClientProgress::Connected => CredentialPhase::Handshaking,
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn push_event(&self, event: LiveEvent) {
        let mut state = self.lock_state();
        if state.phase == CredentialPhase::Closed {
            return;
        }
        if matches!(event, LiveEvent::PeerBurned) {
            state.phase = CredentialPhase::PeerBurned;
        } else if state.events.len() < 100 {
            state.events.push_back(event);
        } else {
            state.events.clear();
            state.phase = CredentialPhase::Failed;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fail_transport(&self) -> CredentialPhase {
        let mut state = self.lock_state();
        if state.phase != CredentialPhase::Closed {
            state.material = None;
            state.verification_code = None;
            state.commands = None;
            state.events.clear();
            state.phase = CredentialPhase::Failed;
        }
        state.phase
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fail_with_client_error(&self, error: ClientError) -> CredentialPhase {
        let mut state = self.lock_state();
        if state.phase != CredentialPhase::Closed {
            state.material = None;
            state.verification_code = None;
            state.commands = None;
            state.events.clear();
            state.phase = phase_for_client_error(error);
        }
        state.phase
    }

    /// Logical cancellation, not preemption. A damaged runtime must be
    /// quarantined by the platform supervisor instead of re-entered.
    #[frb(sync)]
    pub fn close(&self) -> CredentialPhase {
        let mut state = self.lock_state();
        state.inputs = None;
        state.material = None;
        state.verification_code = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            state.commands = None;
            state.events.clear();
        }
        #[cfg(target_arch = "wasm32")]
        {
            state.commands.clear();
            state.events.clear();
        }
        state.phase = CredentialPhase::Closed;
        state.phase
    }

    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poison| {
            let mut state = poison.into_inner();
            state.inputs = None;
            state.material = None;
            state.verification_code = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                state.commands = None;
                state.events.clear();
            }
            #[cfg(target_arch = "wasm32")]
            {
                state.commands.clear();
                state.events.clear();
            }
            if state.phase != CredentialPhase::Closed {
                state.phase = CredentialPhase::Failed;
            }
            state
        })
    }
}

pub(crate) fn format_verification_code(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut code = String::with_capacity(14);
    for (index, byte) in hash.iter().take(6).enumerate() {
        if index == 2 || index == 4 {
            code.push(' ');
        }
        code.push(HEX[usize::from(byte >> 4)] as char);
        code.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    code
}

#[cfg(any(target_arch = "wasm32", test))]
fn valid_web_relay_url(value: &str) -> bool {
    if value.is_empty() || value.len() > 2_048 || value.contains(['\r', '\n', '#']) {
        return false;
    }
    if value.starts_with("wss://") {
        return true;
    }
    let Some(authority) = value.strip_prefix("ws://") else {
        return false;
    };
    let host = authority.split('/').next().unwrap_or_default();
    host == "localhost"
        || host.starts_with("localhost:")
        || host == "127.0.0.1"
        || host.starts_with("127.0.0.1:")
        || host == "[::1]"
        || host.starts_with("[::1]:")
}

#[cfg(target_arch = "wasm32")]
const fn phase_for_web_error(error: WebError) -> CredentialPhase {
    match error {
        WebError::Connection | WebError::RelayRejected => CredentialPhase::ConnectionLost,
        WebError::DeliveryTimeout => CredentialPhase::DeliveryTimedOut,
        WebError::Authentication => CredentialPhase::Failed,
        WebError::Cancelled => CredentialPhase::Closed,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn valid_relay_url(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2_048
        && (value.starts_with("ws://") || value.starts_with("wss://"))
        && !value.contains(['\r', '\n', '#'])
}

#[cfg(not(target_arch = "wasm32"))]
const fn phase_for_client_error(error: ClientError) -> CredentialPhase {
    match error {
        ClientError::Connection | ClientError::RelayRejected => CredentialPhase::ConnectionLost,
        ClientError::DeliveryTimeout => CredentialPhase::DeliveryTimedOut,
        ClientError::Authentication | ClientError::InvalidInviteInput => CredentialPhase::Failed,
    }
}

/// Validates and moves a credential pair into a single bounded Rust-owned slot.
/// The caller remains responsible for overwriting the borrowed source bytes.
pub(crate) fn stage_borrowed(room: &[u8], password: &[u8]) -> u32 {
    crate::panic_boundary::contain(|| stage_borrowed_inner(room, password)).unwrap_or_default()
}

pub(crate) fn stage_message_borrowed(message: &[u8]) -> u32 {
    crate::panic_boundary::contain(|| {
        if message.is_empty()
            || message.len() > MAX_MESSAGE_BYTES
            || std::str::from_utf8(message).is_err()
        {
            return 0;
        }
        let mut slot = STAGED_MESSAGE.lock().unwrap_or_else(|poison| {
            let mut slot = poison.into_inner();
            *slot = None;
            slot
        });
        if slot.is_some() {
            return 0;
        }
        let token = next_token();
        *slot = Some(StagedMessage {
            token,
            bytes: Zeroizing::new(message.to_vec()),
        });
        token
    })
    .unwrap_or_default()
}

fn take_staged_message(token: u32) -> Option<Zeroizing<Vec<u8>>> {
    if token == 0 {
        return None;
    }
    let mut slot = STAGED_MESSAGE.lock().unwrap_or_else(|poison| {
        let mut slot = poison.into_inner();
        *slot = None;
        slot
    });
    if slot.as_ref().is_none_or(|staged| staged.token != token) {
        return None;
    }
    slot.take().map(|staged| staged.bytes)
}

fn stage_borrowed_inner(room: &[u8], password: &[u8]) -> u32 {
    let mut slot = lock_staged();
    if slot.is_some() {
        return 0;
    }
    let Ok(room) = numeric_text(room).and_then(|value| RoomCode::parse(value).map_err(|_| ()))
    else {
        return 0;
    };
    let Ok(password) =
        numeric_text(password).and_then(|value| NumericPassword::parse(value).map_err(|_| ()))
    else {
        return 0;
    };
    let token = next_token();
    *slot = Some(StagedInput {
        token,
        inputs: Inputs { room, password },
    });
    token
}

fn take_staged(token: u32) -> Option<Inputs> {
    if token == 0 {
        return None;
    }
    let mut slot = lock_staged();
    if slot.as_ref().is_none_or(|staged| staged.token != token) {
        return None;
    }
    slot.take().map(|staged| staged.inputs)
}

fn lock_staged() -> MutexGuard<'static, Option<StagedInput>> {
    STAGED_INPUT.lock().unwrap_or_else(|poison| {
        let mut slot = poison.into_inner();
        *slot = None;
        slot
    })
}

fn next_token() -> u32 {
    loop {
        let token = NEXT_STAGE_TOKEN.fetch_add(1, Ordering::Relaxed);
        if token != 0 {
            return token;
        }
    }
}

struct DerivationGuard;

impl DerivationGuard {
    fn acquire() -> Result<Self, ()> {
        DERIVATION_RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| ())
    }
}

impl Drop for DerivationGuard {
    fn drop(&mut self) {
        DERIVATION_RUNNING.store(false, Ordering::Release);
    }
}

fn numeric_text(bytes: &[u8]) -> Result<&str, ()> {
    if bytes.is_empty() || bytes.len() > MAX_NUMERIC_DIGITS || !bytes.iter().all(u8::is_ascii_digit)
    {
        return Err(());
    }
    std::str::from_utf8(bytes).map_err(|_| ())
}

#[cfg(test)]
struct InputWipe<'a>(&'a mut [u8]);
#[cfg(test)]
impl Drop for InputWipe<'_> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn verification_code_is_short_grouped_and_deterministic() {
        let mut hash = [0_u8; 32];
        hash[..6].copy_from_slice(&[0x01, 0x23, 0xab, 0xcd, 0xef, 0x45]);
        assert_eq!(format_verification_code(&hash), "0123 ABCD EF45");
    }

    fn cheap(inputs: &Inputs) -> Result<SessionSecret, ()> {
        derive_session_secret(
            &inputs.room,
            &inputs.password,
            PasswordKdfParams {
                memory_kib: 8,
                iterations: 1,
                lanes: 1,
            },
        )
        .map_err(|_| ())
    }

    fn stage(probe: &NativeSession, room: &[u8], password: &[u8]) -> bool {
        let mut room = room.to_vec();
        let mut password = password.to_vec();
        probe.stage_and_wipe(&mut room, &mut password)
    }

    #[test]
    fn lengths_canonicalization_and_matching_core_output() {
        for n in [1, 39, 128] {
            let probe = NativeSession::new();
            let mut room = vec![b'1'; n];
            let mut password = vec![b'2'; n];
            assert!(probe.stage_and_wipe(&mut room, &mut password));
            assert!(room.iter().chain(&password).all(|b| *b == 0));
            assert_eq!(probe.derive_with(cheap), CredentialPhase::Ready);
            assert!(probe.lock_state().inputs.is_none());
            assert_eq!(probe.close(), CredentialPhase::Closed);
            assert!(probe.lock_state().material.is_none());
        }
        let probe = NativeSession::new();
        assert!(stage(&probe, b"00013", b"001370"));
        let expected = cheap(&Inputs {
            room: RoomCode::parse("13").unwrap(),
            password: NumericPassword::parse("1370").unwrap(),
        })
        .unwrap();
        assert_eq!(probe.derive_with(cheap), CredentialPhase::Ready);
        assert_eq!(
            probe
                .lock_state()
                .material
                .as_ref()
                .unwrap()
                .secret
                .expose_bytes(),
            expected.expose_bytes()
        );
    }

    #[test]
    fn rejects_each_invalid_field_and_wipes_both_buffers() {
        for bad in [
            vec![],
            vec![b'0'; 129],
            vec![0xff],
            b"12\n".to_vec(),
            "١٢".as_bytes().to_vec(),
            b"+1".to_vec(),
        ] {
            for bad_password in [false, true] {
                let probe = NativeSession::new();
                let (mut room, mut password) = if bad_password {
                    (b"13".to_vec(), bad.clone())
                } else {
                    (bad.clone(), b"1370".to_vec())
                };
                assert!(!probe.stage_and_wipe(&mut room, &mut password));
                assert!(room.iter().chain(&password).all(|b| *b == 0));
                assert_eq!(probe.status(), CredentialPhase::Failed);
                assert!(probe.lock_state().inputs.is_none());
                assert!(!stage(&probe, b"13", b"1370"));
            }
        }
    }

    #[test]
    fn duplicate_stage_preserves_first_pair_and_close_discards_staged_inputs() {
        let probe = NativeSession::new();
        assert!(stage(&probe, b"0", b"0000"));
        let mut room = b"88".to_vec();
        let mut password = b"99".to_vec();
        assert!(!probe.stage_and_wipe(&mut room, &mut password));
        assert!(room.iter().chain(&password).all(|b| *b == 0));
        assert_eq!(
            probe
                .lock_state()
                .inputs
                .as_ref()
                .unwrap()
                .password
                .expose_bytes(),
            b"0"
        );
        assert_eq!(probe.close(), CredentialPhase::Closed);
        assert!(probe.lock_state().inputs.is_none());
        assert!(!stage(&probe, b"13", b"1370"));
        assert_eq!(
            probe.derive_with(|_| panic!("no work")),
            CredentialPhase::Closed
        );
    }

    #[test]
    fn close_during_work_cannot_install_a_late_key() {
        let probe = Arc::new(NativeSession::new());
        assert!(stage(&probe, b"13", b"1370"));
        let barrier = Arc::new(Barrier::new(2));
        let worker_probe = Arc::clone(&probe);
        let worker_barrier = Arc::clone(&barrier);
        let worker = std::thread::spawn(move || {
            worker_probe.derive_with(|inputs| {
                worker_barrier.wait();
                worker_barrier.wait();
                cheap(inputs)
            })
        });
        barrier.wait();
        assert_eq!(
            probe.derive_with(|_| panic!("duplicate")),
            CredentialPhase::Deriving
        );
        assert_eq!(probe.close(), CredentialPhase::Closed);
        barrier.wait();
        assert_eq!(worker.join().unwrap(), CredentialPhase::Closed);
        assert!(probe.lock_state().material.is_none());
        assert!(probe.lock_state().inputs.is_none());
    }

    #[test]
    fn failure_unwind_poison_and_empty_derive_are_terminal() {
        let empty = NativeSession::new();
        assert_eq!(
            empty.derive_with(|_| panic!("no input")),
            CredentialPhase::Failed
        );
        for panic in [false, true] {
            let probe = NativeSession::new();
            assert!(stage(&probe, b"13", b"1370"));
            assert_eq!(
                probe.derive_with(|_| {
                    if panic {
                        panic!("SCOTOMA_CREDENTIAL_TEST_CANARY");
                    }
                    Err(())
                }),
                CredentialPhase::Failed
            );
            assert!(probe.lock_state().inputs.is_none());
            assert!(probe.lock_state().material.is_none());
        }
        let poisoned = NativeSession::new();
        assert!(stage(&poisoned, b"13", b"1370"));
        let _ = crate::panic_boundary::contain(|| {
            let _guard = poisoned.state.lock().unwrap();
            panic!("SCOTOMA_CREDENTIAL_TEST_CANARY");
        });
        assert_eq!(poisoned.status(), CredentialPhase::Failed);
        assert!(poisoned.lock_state().inputs.is_none());
    }

    #[test]
    fn input_wipe_runs_during_unwind_without_reading_freed_memory() {
        let mut bytes = *b"1370";
        let _ = crate::panic_boundary::contain(|| {
            let _guard = InputWipe(&mut bytes);
            panic!("SCOTOMA_CREDENTIAL_TEST_CANARY");
        });
        assert_eq!(bytes, [0; 4]);
    }

    #[test]
    fn bounded_slot_is_one_shot_and_claim_uses_only_a_token() {
        *lock_staged() = None;
        let token = stage_borrowed(b"00013", b"001370");
        assert_ne!(token, 0);
        assert_eq!(stage_borrowed(b"88", b"99"), 0);
        let wrong = NativeSession::new();
        assert!(!wrong.claim(token.wrapping_add(1)));
        let probe = NativeSession::new();
        assert!(probe.claim(token));
        assert!(!probe.claim(token));
        assert!(lock_staged().is_none());
        assert_eq!(probe.derive_with(cheap), CredentialPhase::Ready);
        assert_eq!(probe.close(), CredentialPhase::Closed);
    }

    #[test]
    fn message_slot_is_bounded_utf8_and_one_shot() {
        *STAGED_MESSAGE.lock().unwrap() = None;
        assert_eq!(stage_message_borrowed(b""), 0);
        assert_eq!(stage_message_borrowed(&[0xff]), 0);
        assert_eq!(
            stage_message_borrowed(&vec![b'a'; MAX_MESSAGE_BYTES + 1]),
            0
        );

        let token = stage_message_borrowed("hola \u{1f510}".as_bytes());
        assert_ne!(token, 0);
        assert_eq!(stage_message_borrowed(b"second"), 0);
        assert!(take_staged_message(token.wrapping_add(1)).is_none());
        let message = take_staged_message(token).expect("matching token");
        assert_eq!(message.as_slice(), "hola \u{1f510}".as_bytes());
        assert!(take_staged_message(token).is_none());
    }

    #[test]
    fn transport_errors_map_to_distinct_terminal_phases() {
        assert_eq!(
            phase_for_client_error(ClientError::Connection),
            CredentialPhase::ConnectionLost
        );
        assert_eq!(
            phase_for_client_error(ClientError::RelayRejected),
            CredentialPhase::ConnectionLost
        );
        assert_eq!(
            phase_for_client_error(ClientError::DeliveryTimeout),
            CredentialPhase::DeliveryTimedOut
        );
        assert_eq!(
            phase_for_client_error(ClientError::Authentication),
            CredentialPhase::Failed
        );
    }

    #[test]
    fn web_relay_url_requires_tls_except_on_loopback() {
        for allowed in [
            "wss://relay.example/v2/connect",
            "ws://localhost:8787/v2/connect",
            "ws://127.0.0.1:8787/v2/connect",
            "ws://[::1]:8787/v2/connect",
        ] {
            assert!(valid_web_relay_url(allowed));
        }
        for denied in [
            "",
            "https://relay.example/v2/connect",
            "ws://relay.example/v2/connect",
            "wss://relay.example/v2/connect#fragment",
            "wss://relay.example/\nsecret",
        ] {
            assert!(!valid_web_relay_url(denied));
        }
    }
}
