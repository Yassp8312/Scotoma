//! TEST-ONLY authenticated TCP loopback operator for sprint 5.5.4.

use ed25519_dalek::SigningKey;
use scotoma_private_rendezvous_v3_model::transport::{
    read_frame, signed_response, verify_request, write_frame, AuthenticatedResponse, ReplayGuard,
};
use std::{
    env, io,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CLIENT_TEST_SEED: [u8; 32] = [0xc1; 32];
const MAX_CLOCK_SKEW_MS: u64 = 5_000;

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 loopback operator failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (operator_id, epoch, timeout_ms) = parse_args()?;
    let signing_key = operator_signing_key(operator_id, epoch);
    let client_key = SigningKey::from_bytes(&CLIENT_TEST_SEED).verifying_key();
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| error.to_string())?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    println!(
        "READY TEST_ONLY {operator_id} {epoch} {address} {}",
        encode_hex(signing_key.verifying_key().to_bytes())
    );

    let timeout = Duration::from_millis(timeout_ms);
    let mut replay_guard = ReplayGuard::default();
    let mut accepted = 0u64;
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        if let Ok(true) = handle_connection(
            &mut stream,
            operator_id,
            epoch,
            &client_key,
            &signing_key,
            &mut replay_guard,
            &mut accepted,
        ) {
            return Ok(());
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: &mut TcpStream,
    operator_id: u16,
    epoch: u64,
    client_key: &ed25519_dalek::VerifyingKey,
    operator_key: &SigningKey,
    replay_guard: &mut ReplayGuard,
    accepted: &mut u64,
) -> Result<bool, ()> {
    let frame = read_frame(stream).map_err(|_| ())?;
    let request = verify_request(&frame, client_key, operator_id, epoch).map_err(|_| ())?;
    replay_guard
        .accept(&request, unix_time_ms().map_err(|_| ())?, MAX_CLOCK_SKEW_MS)
        .map_err(|_| ())?;
    *accepted = accepted.saturating_add(1);

    let (status, payload, quit) = match request.command.as_slice() {
        b"PING" => (0, b"PONG".to_vec(), false),
        b"STATS" => (0, accepted.to_string().into_bytes(), false),
        b"QUIT" => (0, b"BYE".to_vec(), true),
        _ => (1, b"REJECT".to_vec(), false),
    };
    let response = signed_response(
        operator_key,
        &AuthenticatedResponse {
            operator_id,
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

fn parse_args() -> Result<(u16, u64, u64), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [operator_flag, operator, epoch_flag, epoch, timeout_flag, timeout] = args.as_slice()
    else {
        return Err("usage: --operator <u16> --epoch <u64> --timeout-ms <u64>".to_owned());
    };
    if operator_flag != "--operator" || epoch_flag != "--epoch" || timeout_flag != "--timeout-ms" {
        return Err("invalid arguments".to_owned());
    }
    let timeout = timeout
        .parse::<u64>()
        .map_err(|_| "invalid timeout".to_owned())?;
    if timeout == 0 || timeout > 60_000 {
        return Err("timeout outside bounds".to_owned());
    }
    Ok((
        operator
            .parse()
            .map_err(|_| "invalid operator".to_owned())?,
        epoch.parse().map_err(|_| "invalid epoch".to_owned())?,
        timeout,
    ))
}

fn operator_signing_key(operator_id: u16, epoch: u64) -> SigningKey {
    let mut seed = [0x91; 32];
    seed[..2].copy_from_slice(&operator_id.to_be_bytes());
    seed[2..10].copy_from_slice(&epoch.to_be_bytes());
    SigningKey::from_bytes(&seed)
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
