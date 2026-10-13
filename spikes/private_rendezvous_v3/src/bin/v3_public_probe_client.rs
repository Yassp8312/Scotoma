//! RESEARCH-ONLY verifier for public v3 operator availability receipts.

use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    read_frame, signed_request, verify_response, write_frame, AuthenticatedRequest,
};
use std::{
    env, fs,
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const KEY_ENVIRONMENT_VARIABLE: &str = "SCOTOMA_V3_PROBE_CLIENT_SEED_FILE";

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 public probe failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args()?;
    let client_key = load_signing_key(KEY_ENVIRONMENT_VARIABLE)?;
    let operator_key = VerifyingKey::from_bytes(&decode_hex_32(&config.operator_key)?)
        .map_err(|_| "invalid operator public key".to_owned())?;
    let request = signed_request(
        &client_key,
        &AuthenticatedRequest {
            operator_id: config.operator_id,
            epoch: config.epoch,
            sequence: config.sequence,
            issued_at_ms: unix_time_ms()?,
            command: b"PING".to_vec(),
        },
    )
    .map_err(|error| format!("could not sign request: {error:?}"))?;

    let started = Instant::now();
    let mut stream = connect_endpoint(&config.endpoint, config.timeout)?;
    stream
        .set_read_timeout(Some(config.timeout))
        .and_then(|()| stream.set_write_timeout(Some(config.timeout)))
        .map_err(|_| "could not set probe timeout".to_owned())?;
    write_frame(&mut stream, &request).map_err(|error| format!("write failed: {error:?}"))?;
    let response = read_frame(&mut stream).map_err(|error| format!("read failed: {error:?}"))?;
    let response = verify_response(
        &response,
        &operator_key,
        config.operator_id,
        config.epoch,
        config.sequence,
    )
    .map_err(|error| format!("invalid signed receipt: {error:?}"))?;
    if response.status != 0 || !response.payload.starts_with(b"PONG ") {
        return Err("operator returned an invalid probe payload".to_owned());
    }
    let accepted = std::str::from_utf8(&response.payload[5..])
        .map_err(|_| "operator returned a non-text probe counter".to_owned())?
        .parse::<u64>()
        .map_err(|_| "operator returned an invalid probe counter".to_owned())?;
    if accepted == 0 {
        return Err("operator returned a zero probe counter".to_owned());
    }
    let latency_micros = started.elapsed().as_micros();
    println!(
        "{{\"schema\":\"scotoma-v3-public-probe-v1\",\"operator\":{},\"epoch\":{},\"sequence\":{},\"accepted\":{},\"latency_micros\":{},\"result\":\"pass\"}}",
        config.operator_id, config.epoch, config.sequence, accepted, latency_micros
    );
    Ok(())
}

struct Config {
    operator_id: u16,
    epoch: u64,
    endpoint: String,
    operator_key: String,
    sequence: u64,
    timeout: Duration,
}

fn parse_args() -> Result<Config, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [operator_flag, operator, epoch_flag, epoch, endpoint_flag, endpoint, key_flag, operator_key, sequence_flag, sequence, timeout_flag, timeout] =
        args.as_slice()
    else {
        return Err(usage());
    };
    if operator_flag != "--operator"
        || epoch_flag != "--epoch"
        || endpoint_flag != "--endpoint"
        || key_flag != "--operator-key"
        || sequence_flag != "--sequence"
        || timeout_flag != "--timeout-ms"
    {
        return Err(usage());
    }
    let timeout = timeout
        .parse::<u64>()
        .map_err(|_| "invalid timeout".to_owned())?;
    if !(100..=30_000).contains(&timeout) {
        return Err("timeout outside 100..=30000 ms".to_owned());
    }
    decode_hex_32(operator_key)?;
    Ok(Config {
        operator_id: operator
            .parse()
            .map_err(|_| "invalid operator".to_owned())?,
        epoch: epoch.parse().map_err(|_| "invalid epoch".to_owned())?,
        endpoint: endpoint.to_owned(),
        operator_key: operator_key.to_owned(),
        sequence: sequence
            .parse()
            .map_err(|_| "invalid sequence".to_owned())?,
        timeout: Duration::from_millis(timeout),
    })
}

fn usage() -> String {
    "usage: --operator <u16> --epoch <u64> --endpoint <ip:port> --operator-key <hex32> --sequence <u64> --timeout-ms <u64>".to_owned()
}

fn load_signing_key(variable: &str) -> Result<SigningKey, String> {
    let path = env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{variable} must name a local secret file"))?;
    let metadata =
        fs::metadata(&path).map_err(|_| "probe client seed file is unavailable".to_owned())?;
    if !metadata.is_file() || metadata.len() > 128 {
        return Err("probe client seed file has invalid type or size".to_owned());
    }
    let value =
        fs::read_to_string(path).map_err(|_| "probe client seed file is unreadable".to_owned())?;
    Ok(SigningKey::from_bytes(&decode_hex_32(value.trim())?))
}

fn unix_time_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before Unix epoch".to_owned())?;
    u64::try_from(duration.as_millis()).map_err(|_| "system clock overflow".to_owned())
}

fn connect_endpoint(endpoint: &str, timeout: Duration) -> Result<TcpStream, String> {
    let addresses = endpoint
        .to_socket_addrs()
        .map_err(|_| "operator endpoint could not be resolved".to_owned())?;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect_timeout(&address, timeout) {
            return Ok(stream);
        }
    }
    Err("operator unavailable".to_owned())
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
