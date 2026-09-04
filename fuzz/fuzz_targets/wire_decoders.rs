#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_protocol::{
    Frame, JoinRequest, NumericPassword, ProtocolLimits, PuzzleSolution, RelayEvent, RoomCode,
};

fuzz_target!(|data: &[u8]| {
    // These functions form the untrusted network and credential parsing boundary.
    // Every input must return a bounded result or an error, never panic or allocate
    // from an attacker-controlled declared length.
    let _ = Frame::decode(data, ProtocolLimits::default());
    let _ = JoinRequest::decode(data);
    let _ = PuzzleSolution::decode(data);
    let _ = RelayEvent::decode(data);

    if let Ok(text) = std::str::from_utf8(data) {
        let _ = RoomCode::parse(text);
        let _ = NumericPassword::parse(text);
    }
});
