#![no_main]

use libfuzzer_sys::fuzz_target;
use scotoma_taceo_client_compile_probe::{
    build_noise_prologue, derive_candidate_keys, map_credential_to_rfc9380_query,
};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let split = usize::from(data[0]) % data.len();
    let room = &data[1..split.max(1)];
    let password = &data[split.max(1)..];
    let Ok(query) = map_credential_to_rfc9380_query(room, password) else {
        return;
    };

    let manifest = [data[0]; 32];
    let epoch = u64::from(data.get(1).copied().unwrap_or_default());
    let keys = derive_candidate_keys(&query, &manifest, epoch);
    let prologue = build_noise_prologue(
        &manifest,
        epoch,
        keys.rendezvous_tag(),
        &[0x11; 32],
        &[0x22; 32],
    );
    let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
        .parse()
        .expect("fixed suite parses");
    let mut initiator = snow::Builder::new(params)
        .prologue(&prologue)
        .expect("fixed prologue is valid")
        .psk(0, keys.noise_psk())
        .expect("fixed-size PSK is valid")
        .build_initiator()
        .expect("fixed suite builds");
    let mut frame = [0_u8; 512];
    let length = initiator
        .write_message(&[], &mut frame)
        .expect("fixed output buffer is sufficient");

    let mutation = &data[2..];
    for (target, source) in frame[..length].iter_mut().zip(mutation.iter()) {
        *target ^= *source;
    }
    let params = "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
        .parse()
        .expect("fixed suite parses");
    let mut responder = snow::Builder::new(params)
        .prologue(&prologue)
        .expect("fixed prologue is valid")
        .psk(0, keys.noise_psk())
        .expect("fixed-size PSK is valid")
        .build_responder()
        .expect("fixed suite builds");
    let mut plaintext = [0_u8; 512];
    let truncated = length.saturating_sub(usize::from(data[1]) % (length + 1));
    let _ = responder.read_message(&frame[..truncated], &mut plaintext);
});
