//! RESEARCH-ONLY public availability probe for an independently operated v3 node.
//!
//! This process does not implement rendezvous, TOPRF or a production ceremony.

use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    read_frame, signed_response, verify_request, write_frame, AuthenticatedResponse, ReplayGuard,
};
use std::{
    env, fs, io,
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const KEY_ENVIRONMENT_VARIABLE: &str = "SCOTOMA_V3_OPERATOR_SEED_FILE";
const MAX_CLOCK_SKEW_MS: u64 = 30_000;

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 public probe operator failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    let signing_key = load_signing_key(KEY_ENVIRONMENT_VARIABLE)?;
    let client_key = VerifyingKey::from_bytes(&decode_hex_32(&config.client_key)?)
        .map_err(|_| "invalid probe client public key".to_owned())?;
    let listener = TcpListener::bind(config.bind).map_err(|error| error.to_string())?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    println!(
        "READY RESEARCH_ONLY {} {} {} {}",
        config.operator_id,
        config.epoch,
        address,
        encode_hex(signing_key.verifying_key().to_bytes())
    );

    let timeout = Duration::from_millis(config.timeout_ms);
    let mut replay_guard = ReplayGuard::default();
    let mut accepted = 0_u64;
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        let _ = handle_connection(
            &mut stream,
            config.operator_id,
            config.epoch,
            &client_key,
            &signing_key,
            &mut replay_guard,
            &mut accepted,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: &mut TcpStream,
    operator_id: u16,
    epoch: u64,
    client_key: &VerifyingKey,
    operator_key: &SigningKey,
    replay_guard: &mut ReplayGuard,
    accepted: &mut u64,
) -> Result<(), ()> {
    let frame = read_frame(stream).map_err(|_| ())?;
    let request = verify_request(&frame, client_key, operator_id, epoch).map_err(|_| ())?;
    replay_guard
        .accept(&request, unix_time_ms().map_err(|_| ())?, MAX_CLOCK_SKEW_MS)
        .map_err(|_| ())?;
    if request.command.as_slice() != b"PING" {
        return Err(());
    }
    *accepted = accepted.saturating_add(1);
    let response = signed_response(
        operator_key,
        &AuthenticatedResponse {
            operator_id,
            epoch,
            request_sequence: request.sequence,
            status: 0,
            payload: format!("PONG {accepted}").into_bytes(),
        },
    )
    .map_err(|_| ())?;
    write_frame(stream, &response).map_err(|_| ())
}

struct Config {
    operator_id: u16,
    epoch: u64,
    bind: SocketAddr,
    client_key: String,
    timeout_ms: u64,
}

fn parse_args() -> Result<Config, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [operator_flag, operator, epoch_flag, epoch, bind_flag, bind, client_flag, client_key, timeout_flag, timeout] =
        args.as_slice()
    else {
        return Err(usage());
    };
    if operator_flag != "--operator"
        || epoch_flag != "--epoch"
        || bind_flag != "--bind"
        || client_flag != "--client-key"
        || timeout_flag != "--timeout-ms"
    {
        return Err(usage());
    }
    let timeout_ms = timeout
        .parse::<u64>()
        .map_err(|_| "invalid timeout".to_owned())?;
    if !(100..=10_000).contains(&timeout_ms) {
        return Err("timeout outside 100..=10000 ms".to_owned());
    }
    decode_hex_32(client_key)?;
    Ok(Config {
        operator_id: operator
            .parse()
            .map_err(|_| "invalid operator".to_owned())?,
        epoch: epoch.parse().map_err(|_| "invalid epoch".to_owned())?,
        bind: bind
            .parse()
            .map_err(|_| "invalid bind address".to_owned())?,
        client_key: client_key.to_owned(),
        timeout_ms,
    })
}

fn usage() -> String {
    "usage: --operator <u16> --epoch <u64> --bind <ip:port> --client-key <hex32> --timeout-ms <u64>"
        .to_owned()
}

fn load_signing_key(variable: &str) -> Result<SigningKey, String> {
    let path = env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{variable} must name a mounted secret file"))?;
    let metadata =
        fs::metadata(&path).map_err(|_| "operator seed file is unavailable".to_owned())?;
    if !metadata.is_file() || metadata.len() > 128 {
        return Err("operator seed file has invalid type or size".to_owned());
    }
    let value =
        fs::read_to_string(path).map_err(|_| "operator seed file is unreadable".to_owned())?;
    Ok(SigningKey::from_bytes(&decode_hex_32(value.trim())?))
}

fn unix_time_ms() -> io::Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    u64::try_from(duration.as_millis()).map_err(io::Error::other)
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.is_ascii() {
        return Err("expected exactly 32 hexadecimal bytes".to_owned());
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid hexadecimal key".to_owned())?;
    }
    Ok(output)
}

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
