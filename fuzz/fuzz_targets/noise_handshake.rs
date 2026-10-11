#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_protocol::{
    HandshakeRole, HandshakeSession, NumericPassword, PasswordKdfParams, RoomCode,
    derive_rendezvous_tag, derive_session_secret,
};

fuzz_target!(|data: &[u8]| {
    let room = RoomCode::parse("424242").expect("fixed room is valid");
    let password = NumericPassword::parse("737373").expect("fixed password is valid");
    let secret = derive_session_secret(
        &room,
        &password,
        PasswordKdfParams {
            memory_kib: 8,
            iterations: 1,
            lanes: 1,
        },
    )
    .expect("minimum fixed fuzz KDF parameters are valid");
    let tag = derive_rendezvous_tag(&room, &secret).expect("fixed tag derivation succeeds");
    let mode = data.first().copied().unwrap_or(0) % 4;
    let hostile = data.get(1..).unwrap_or_default();

    let mut initiator = HandshakeSession::new(HandshakeRole::Initiator, &room, &secret, &tag)
        .expect("fixed initiator initializes");
    let mut responder = HandshakeSession::new(HandshakeRole::Responder, &room, &secret, &tag)
        .expect("fixed responder initializes");

    match mode {
        0 => {
            let _ = responder.responder_receive_initial(hostile);
        }
        1 => {
            let first = initiator.initiator_start().expect("valid first message");
            let _ = responder
                .responder_receive_initial(&first)
                .and_then(|_| initiator.initiator_receive_response(hostile));
        }
        2 => {
            let first = initiator.initiator_start().expect("valid first message");
            let second = responder
                .responder_receive_initial(&first)
                .expect("valid second message");
            let _ = initiator
                .initiator_receive_response(&second)
                .and_then(|_| responder.responder_receive_confirmation(hostile));
        }
        _ => {
            let first = initiator.initiator_start().expect("valid first message");
            let second = responder
                .responder_receive_initial(&first)
                .expect("valid second message");
            let third = initiator
                .initiator_receive_response(&second)
                .expect("valid third message");
            let fourth = responder
                .responder_receive_confirmation(&third)
                .expect("valid fourth message");
            let _ = initiator.initiator_receive_confirmation(if hostile.is_empty() {
                &fourth
            } else {
                hostile
            });
        }
    }
});
