//! TEST-ONLY authenticated multiprocess ceremony coordinator.

use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::{
    ceremony::{CeremonyConfig, CeremonyEnvelope, CeremonyMessage, CeremonyPhase, NetworkCeremony},
    transport::{
        read_frame, signed_response, verify_request, write_frame, AuthenticatedRequest,
        AuthenticatedResponse, ReplayGuard,
    },
};
use std::{
    env, io,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_CLOCK_SKEW_MS: u64 = 5_000;
const PARTICIPANT_COUNT: usize = 3;

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 ceremony coordinator failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (coordinator_id, epoch, timeout_ms) = parse_args()?;
    let signing_key = coordinator_signing_key(coordinator_id, epoch);
    let participant_keys =
        std::array::from_fn(|id| participant_signing_key(id as u16).verifying_key());
    let mut replay_guards = [ReplayGuard::default(); PARTICIPANT_COUNT];
    let mut ceremony = NetworkCeremony::new(CeremonyConfig {
        ceremony_id: [0x43; 16],
        manifest_hash: [0x53; 32],
        epoch,
        previous_epoch: epoch.checked_sub(1),
        dealer_count: 2,
        recipient_count: 3,
        threshold: 2,
    })
    .map_err(|error| format!("invalid ceremony: {error:?}"))?;

    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| error.to_string())?;
    println!(
        "READY TEST_ONLY {coordinator_id} {epoch} {} {}",
        listener.local_addr().map_err(|error| error.to_string())?,
        encode_hex(signing_key.verifying_key().to_bytes())
    );
    let timeout = Duration::from_millis(timeout_ms);

    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        if let Ok(true) = handle_connection(
            &mut stream,
            coordinator_id,
            epoch,
            &participant_keys,
            &signing_key,
            &mut replay_guards,
            &mut ceremony,
        ) {
            return Ok(());
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: &mut TcpStream,
    coordinator_id: u16,
    epoch: u64,
    participant_keys: &[VerifyingKey; PARTICIPANT_COUNT],
    coordinator_key: &SigningKey,
    replay_guards: &mut [ReplayGuard; PARTICIPANT_COUNT],
    ceremony: &mut NetworkCeremony,
) -> Result<bool, ()> {
    let frame = read_frame(stream).map_err(|_| ())?;
    let (sender_id, request) = participant_keys
        .iter()
        .enumerate()
        .find_map(|(sender, key)| {
            verify_request(&frame, key, coordinator_id, epoch)
                .ok()
                .map(|request| (sender as u16, request))
        })
        .ok_or(())?;
    replay_guards[usize::from(sender_id)]
        .accept(&request, unix_time_ms().map_err(|_| ())?, MAX_CLOCK_SKEW_MS)
        .map_err(|_| ())?;

    let (status, payload, quit) = execute_command(ceremony, sender_id, &request)?;
    let response = signed_response(
        coordinator_key,
        &AuthenticatedResponse {
            operator_id: coordinator_id,
            epoch,
            request_sequence: request.sequence,
            status,
            payload,
        },
    )
    .map_err(|_| ())?;
    write_frame(stream, &response).map_err(|_| ())?;
    Ok(quit)
}

fn execute_command(
    ceremony: &mut NetworkCeremony,
    sender_id: u16,
    request: &AuthenticatedRequest,
) -> Result<(u8, Vec<u8>, bool), ()> {
    let text = std::str::from_utf8(&request.command).map_err(|_| ())?;
    let fields = text.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.as_slice() == ["STATUS"] {
        return Ok((0, phase_report(ceremony), false));
    }
    if fields.as_slice() == ["TEST_TIMEOUT"] {
        let result = ceremony.timeout();
        return Ok(result_report(ceremony, result.err(), false));
    }
    if fields.as_slice() == ["QUIT"] {
        return Ok((0, b"BYE".to_vec(), true));
    }

    let sequence = u32::try_from(request.sequence).map_err(|_| ())?;
    let message = match fields.as_slice() {
        ["COMMIT", digest, proof] => CeremonyMessage::Commitment {
            digest: [parse_byte(digest)?; 32],
            proof_digest: [parse_byte(proof)?; 32],
            proof_valid: parse_byte(proof)? != 0,
        },
        ["SHARE", recipient, commitment, ciphertext] => CeremonyMessage::EncryptedShare {
            recipient_id: parse_u16(recipient)?,
            polynomial_commitment_digest: [parse_byte(commitment)?; 32],
            ciphertext_digest: [parse_byte(ciphertext)?; 32],
        },
        ["VERIFY", dealer, commitment, ciphertext, evidence] => {
            CeremonyMessage::ShareVerification {
                dealer_id: parse_u16(dealer)?,
                polynomial_commitment_digest: [parse_byte(commitment)?; 32],
                ciphertext_digest: [parse_byte(ciphertext)?; 32],
                share_valid: true,
                evidence_digest: [parse_byte(evidence)?; 32],
                evidence_valid: parse_byte(evidence)? != 0,
            }
        }
        ["COMPLAIN", dealer, commitment, ciphertext, evidence] => {
            CeremonyMessage::ShareVerification {
                dealer_id: parse_u16(dealer)?,
                polynomial_commitment_digest: [parse_byte(commitment)?; 32],
                ciphertext_digest: [parse_byte(ciphertext)?; 32],
                share_valid: false,
                evidence_digest: [parse_byte(evidence)?; 32],
                evidence_valid: parse_byte(evidence)? != 0,
            }
        }
        _ => return Err(()),
    };
    let result = ceremony.accept(CeremonyEnvelope {
        ceremony_id: [0x43; 16],
        manifest_hash: [0x53; 32],
        epoch: request.epoch,
        sender_id,
        sequence,
        authenticated: true,
        message,
    });
    Ok(result_report(ceremony, result.err(), false))
}

fn result_report(
    ceremony: &NetworkCeremony,
    error: Option<scotoma_private_rendezvous_v3_model::ceremony::CeremonyError>,
    quit: bool,
) -> (u8, Vec<u8>, bool) {
    let status = u8::from(error.is_some());
    let mut report = phase_report(ceremony);
    if let Some(error) = error {
        report.extend_from_slice(format!(" ERROR {error:?}").as_bytes());
    }
    (status, report, quit)
}

fn phase_report(ceremony: &NetworkCeremony) -> Vec<u8> {
    let phase = match ceremony.phase() {
        CeremonyPhase::Commitments => "COMMITMENTS",
        CeremonyPhase::Shares => "SHARES",
        CeremonyPhase::Verification => "VERIFICATION",
        CeremonyPhase::Ready => "READY",
        CeremonyPhase::Terminal => "TERMINAL",
    };
    format!(
        "PHASE {phase} COMMITMENTS {} SHARES {} VERIFICATIONS {}",
        ceremony.commitment_count(),
        ceremony.share_count(),
        ceremony.verification_count()
    )
    .into_bytes()
}

fn parse_args() -> Result<(u16, u64, u64), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [id_flag, id, epoch_flag, epoch, timeout_flag, timeout] = args.as_slice() else {
        return Err("usage: --coordinator <u16> --epoch <u64> --timeout-ms <u64>".to_owned());
    };
    if id_flag != "--coordinator" || epoch_flag != "--epoch" || timeout_flag != "--timeout-ms" {
        return Err("invalid arguments".to_owned());
    }
    let timeout = timeout.parse().map_err(|_| "invalid timeout".to_owned())?;
    if timeout == 0 || timeout > 60_000 {
        return Err("timeout outside bounds".to_owned());
    }
    Ok((
        id.parse().map_err(|_| "invalid coordinator".to_owned())?,
        epoch.parse().map_err(|_| "invalid epoch".to_owned())?,
        timeout,
    ))
}

fn participant_signing_key(participant_id: u16) -> SigningKey {
    let mut seed = [0x71; 32];
    seed[..2].copy_from_slice(&participant_id.to_be_bytes());
    SigningKey::from_bytes(&seed)
}

fn coordinator_signing_key(coordinator_id: u16, epoch: u64) -> SigningKey {
    let mut seed = [0x81; 32];
    seed[..2].copy_from_slice(&coordinator_id.to_be_bytes());
    seed[2..10].copy_from_slice(&epoch.to_be_bytes());
    SigningKey::from_bytes(&seed)
}

fn parse_byte(value: &str) -> Result<u8, ()> {
    value.parse().map_err(|_| ())
}

fn parse_u16(value: &str) -> Result<u16, ()> {
    value.parse().map_err(|_| ())
}

fn unix_time_ms() -> io::Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    u64::try_from(duration.as_millis()).map_err(io::Error::other)
}

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
