//! Candidate transfer boundary. PUBLIC TEST INPUTS ONLY until glue/lifetime
//! gates are resolved. No networking, credential getters, key export or retry.

#[cfg(not(target_arch = "wasm32"))]
use std::collections::VecDeque;
use std::sync::{
    Mutex, MutexGuard,
    atomic::{AtomicU32, Ordering},
};

use flutter_rust_bridge::frb;
#[cfg(not(target_arch = "wasm32"))]
use scotoma_client::{
    ClientProgress, DerivedClientConfig, LiveCommand, LiveEvent, LiveExit, authenticate_derived,
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
    #[cfg(not(target_arch = "wasm32"))]
    commands: Option<tokio::sync::mpsc::Sender<LiveCommand>>,
    #[cfg(not(target_arch = "wasm32"))]
    events: VecDeque<LiveEvent>,
}

struct ReadyMaterial {
    room: RoomCode,
    secret: SessionSecret,
}

/// One-shot public-input experiment, NOT the production session API.
#[frb(opaque)]
pub struct CredentialProbe {
    state: Mutex<State>,
}

impl CredentialProbe {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: CredentialPhase::Empty,
                inputs: None,
                material: None,
                #[cfg(not(target_arch = "wasm32"))]
                commands: None,
                #[cfg(not(target_arch = "wasm32"))]
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
            let _permit = super::benchmark::RunGuard::acquire().map_err(|_| ())?;
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

    /// Claims a message staged through the bounded platform entrypoint and
    /// queues it for authenticated encryption. Only the token crosses FRB.
    #[frb(sync)]
    pub fn claim_message(&self, token: u32) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = take_staged_message(token);
            return false;
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
        return None;
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
        return false;
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
        {
            let _ = relay_url;
            let mut state = self.lock_state();
            if state.phase != CredentialPhase::Closed {
                if let Some(ReadyMaterial { room, secret }) = state.material.take() {
                    drop(room);
                    drop(secret);
                }
                state.phase = CredentialPhase::Failed;
            }
            return state.phase;
        }

        #[cfg(not(target_arch = "wasm32"))]
        self.run_native_transport(relay_url).await
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
                Err(_) => return self.fail_transport(),
            },
            () = self.wait_for_close() => return CredentialPhase::Closed,
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(2);
        {
            let mut state = self.lock_state();
            if state.phase == CredentialPhase::Closed {
                return CredentialPhase::Closed;
            }
            state.commands = Some(sender);
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
            Err(_) => {
                state.events.clear();
                state.phase = CredentialPhase::Failed;
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
            state.commands = None;
            state.events.clear();
            state.phase = CredentialPhase::Failed;
        }
        state.phase
    }

    /// Logical cancellation, not preemption. Do NOT enter a poisoned Wasm
    /// runtime; a production web supervisor remains a separate gate.
    #[frb(sync)]
    pub fn close(&self) -> CredentialPhase {
        let mut state = self.lock_state();
        state.inputs = None;
        state.material = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            state.commands = None;
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
            #[cfg(not(target_arch = "wasm32"))]
            {
                state.commands = None;
                state.events.clear();
            }
            if state.phase != CredentialPhase::Closed {
                state.phase = CredentialPhase::Failed;
            }
            state
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn valid_relay_url(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2_048
        && (value.starts_with("ws://") || value.starts_with("wss://"))
        && !value.contains(['\r', '\n', '#'])
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

impl Default for CredentialProbe {
    fn default() -> Self {
        Self::new()
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

    fn stage(probe: &CredentialProbe, room: &[u8], password: &[u8]) -> bool {
        let mut room = room.to_vec();
        let mut password = password.to_vec();
        probe.stage_and_wipe(&mut room, &mut password)
    }

    #[test]
    fn lengths_canonicalization_and_matching_core_output() {
        for n in [1, 39, 128] {
            let probe = CredentialProbe::new();
            let mut room = vec![b'1'; n];
            let mut password = vec![b'2'; n];
            assert!(probe.stage_and_wipe(&mut room, &mut password));
            assert!(room.iter().chain(&password).all(|b| *b == 0));
            assert_eq!(probe.derive_with(cheap), CredentialPhase::Ready);
            assert!(probe.lock_state().inputs.is_none());
            assert_eq!(probe.close(), CredentialPhase::Closed);
            assert!(probe.lock_state().material.is_none());
        }
        let probe = CredentialProbe::new();
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
                let probe = CredentialProbe::new();
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
        let probe = CredentialProbe::new();
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
        let probe = Arc::new(CredentialProbe::new());
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
        let empty = CredentialProbe::new();
        assert_eq!(
            empty.derive_with(|_| panic!("no input")),
            CredentialPhase::Failed
        );
        for panic in [false, true] {
            let probe = CredentialProbe::new();
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
        let poisoned = CredentialProbe::new();
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
        let wrong = CredentialProbe::new();
        assert!(!wrong.claim(token.wrapping_add(1)));
        let probe = CredentialProbe::new();
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
}
