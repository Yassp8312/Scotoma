//! RESEARCH-ONLY request/receipt helper for HTTPS-hosted public probes.
//!
//! The surrounding PowerShell runner performs HTTPS. This helper exclusively
//! signs request bodies and verifies signed response bodies.

use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    signed_request, verify_response, AuthenticatedRequest, MAX_AUTHENTICATED_FRAME_BYTES,
};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const KEY_ENVIRONMENT_VARIABLE: &str = "SCOTOMA_V3_PROBE_CLIENT_SEED_FILE";

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 public HTTP probe client failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("request") => make_request(&args[1..]),
        Some("verify") => verify_receipt(&args[1..]),
        _ => Err(usage()),
    }
}

fn make_request(args: &[String]) -> Result<(), String> {
    let (operator_id, epoch, sequence, remainder) = parse_context(args)?;
    if !remainder.is_empty() {
        return Err(usage());
    }
    let client_key = load_signing_key(KEY_ENVIRONMENT_VARIABLE)?;
    let frame = signed_request(
        &client_key,
        &AuthenticatedRequest {
            operator_id,
            epoch,
            sequence,
            issued_at_ms: unix_time_ms()?,
            command: b"PING".to_vec(),
        },
    )
    .map_err(|error| format!("could not sign request: {error:?}"))?;
    println!("{}", encode_hex(&frame));
    Ok(())
}

fn verify_receipt(args: &[String]) -> Result<(), String> {
    let (operator_id, epoch, sequence, remainder) = parse_context(args)?;
    let [key_flag, operator_key, response_flag, response_hex] = remainder else {
        return Err(usage());
    };
    if key_flag != "--operator-key" || response_flag != "--response-hex" {
        return Err(usage());
    }
    let operator_key = VerifyingKey::from_bytes(&decode_hex_32(operator_key)?)
        .map_err(|_| "invalid operator public key".to_owned())?;
    let frame = decode_hex(response_hex)?;
    let receipt_hash = encode_hex(Sha256::digest(&frame));
    let response = verify_response(&frame, &operator_key, operator_id, epoch, sequence)
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
    println!(
        "{{\"schema\":\"scotoma-v3-public-http-probe-v1\",\"operator\":{operator_id},\"epoch\":{epoch},\"sequence\":{sequence},\"accepted\":{accepted},\"receipt_sha256\":\"{receipt_hash}\",\"result\":\"pass\"}}"
    );
    Ok(())
}

fn parse_context(args: &[String]) -> Result<(u16, u64, u64, &[String]), String> {
    let [operator_flag, operator, epoch_flag, epoch, sequence_flag, sequence, remainder @ ..] =
        args
    else {
        return Err(usage());
    };
    if operator_flag != "--operator" || epoch_flag != "--epoch" || sequence_flag != "--sequence" {
        return Err(usage());
    }
    Ok((
        operator
            .parse()
            .map_err(|_| "invalid operator".to_owned())?,
        epoch.parse().map_err(|_| "invalid epoch".to_owned())?,
        sequence
            .parse()
            .map_err(|_| "invalid sequence".to_owned())?,
        remainder,
    ))
}

fn usage() -> String {
    "usage: request --operator <u16> --epoch <u64> --sequence <u64> OR verify --operator <u16> --epoch <u64> --sequence <u64> --operator-key <hex32> --response-hex <hex>".to_owned()
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
