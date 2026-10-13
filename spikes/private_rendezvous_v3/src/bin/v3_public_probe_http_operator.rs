//! RESEARCH-ONLY HTTP availability probe for free web-service hosts.
//!
//! HTTPS is terminated by the hosting platform. The signed request and receipt
//! remain bound to operator, epoch and sequence inside the HTTP body.

use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    signed_response, verify_request, AuthenticatedResponse, ReplayGuard,
    MAX_AUTHENTICATED_FRAME_BYTES,
};
use std::{
    env, fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const KEY_ENVIRONMENT_VARIABLE: &str = "SCOTOMA_V3_OPERATOR_SEED_FILE";
const MAX_CLOCK_SKEW_MS: u64 = 30_000;
const MAX_HEADER_BYTES: usize = 4_096;

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 public HTTP probe operator failed: {error}");
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
        "READY RESEARCH_ONLY HTTP {} {} {} {}",
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
        if handle_connection(
            &mut stream,
            config.operator_id,
            config.epoch,
            &client_key,
            &signing_key,
            &mut replay_guard,
            &mut accepted,
        )
        .is_err()
        {
            let _ = write_http_response(&mut stream, 400, "invalid probe");
        }
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
    let (request_line, content_length) = read_http_headers(stream)?;
    if request_line == "GET /healthz HTTP/1.1" {
        return write_http_response(stream, 200, "RESEARCH_ONLY").map_err(|_| ());
    }
    if request_line != "POST /probe HTTP/1.1" {
        return Err(());
    }
    let content_length = content_length.ok_or(())?;
    if content_length == 0 || content_length > MAX_AUTHENTICATED_FRAME_BYTES * 2 {
        return Err(());
    }
    let mut body = vec![0_u8; content_length];
    stream.read_exact(&mut body).map_err(|_| ())?;
    let body = std::str::from_utf8(&body).map_err(|_| ())?;
    let frame = decode_hex(body).map_err(|_| ())?;
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
    write_http_response(stream, 200, &encode_hex(response)).map_err(|_| ())
}

fn read_http_headers(stream: &mut TcpStream) -> Result<(String, Option<usize>), ()> {
    let mut bytes = Vec::with_capacity(512);
    let mut byte = [0_u8; 1];
    while bytes.len() < MAX_HEADER_BYTES {
        stream.read_exact(&mut byte).map_err(|_| ())?;
        bytes.push(byte[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            let text = std::str::from_utf8(&bytes).map_err(|_| ())?;
            let mut lines = text.split("\r\n");
            let request_line = lines.next().ok_or(())?.to_owned();
            let mut content_length = None;
            for line in lines.filter(|line| !line.is_empty()) {
                let (name, value) = line.split_once(':').ok_or(())?;
                if name.eq_ignore_ascii_case("content-length") {
                    if content_length.is_some() {
                        return Err(());
                    }
                    content_length = Some(value.trim().parse::<usize>().map_err(|_| ())?);
                }
            }
            return Ok((request_line, content_length));
        }
    }
    Err(())
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
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
    if args.as_slice() == ["--render-env"] {
        let port = required_env("PORT")?;
        return parse_config(
            &required_env("SCOTOMA_V3_OPERATOR_ID")?,
            &required_env("SCOTOMA_V3_EPOCH")?,
            &format!("0.0.0.0:{port}"),
            &required_env("SCOTOMA_V3_PROBE_CLIENT_PUBLIC_KEY")?,
            &env::var("SCOTOMA_V3_TIMEOUT_MS").unwrap_or_else(|_| "1500".to_owned()),
        );
    }
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
    parse_config(operator, epoch, bind, client_key, timeout)
}

fn parse_config(
    operator: &str,
    epoch: &str,
    bind: &str,
    client_key: &str,
    timeout: &str,
) -> Result<Config, String> {
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

fn required_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("missing environment variable {name}"))
}

fn usage() -> String {
    "usage: --render-env OR --operator <u16> --epoch <u64> --bind <ip:port> --client-key <hex32> --timeout-ms <u64>".to_owned()
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

fn unix_time_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before Unix epoch".to_owned())?;
    u64::try_from(duration.as_millis()).map_err(|_| "system clock overflow".to_owned())
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || value.len() > MAX_AUTHENTICATED_FRAME_BYTES * 2
    {
        return Err("invalid hexadecimal frame length".to_owned());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| "invalid hexadecimal frame".to_owned())
        })
        .collect()
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    let bytes = decode_hex(value)?;
    bytes
        .try_into()
        .map_err(|_| "expected exactly 32 hexadecimal bytes".to_owned())
}

fn encode_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
