//! TEST-ONLY line-protocol process for exercising v3 admission boundaries.

use scotoma_private_rendezvous_v3_model::operational::{
    AdmissionConfig, AdmissionController, AdmissionDecision, PuzzleProof,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    io::{self, Read, Write},
};

const POW_DOMAIN: &[u8] = b"SCOTOMA-v3-test-only-pow";
const MAX_COMMAND_BYTES: usize = 256;

fn main() {
    if let Err(error) = run() {
        eprintln!("v3 operational node failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (operator_id, epoch) = parse_args()?;
    let config = AdmissionConfig {
        operator_id,
        epoch,
        normal_attempts_per_connection: 2,
        network_attempts_per_window: 4,
        global_attempts_per_window: 8,
        max_connections: 16,
        max_networks: 16,
        max_challenges: 4,
        base_difficulty: 4,
        max_difficulty: 12,
        challenge_ttl_ticks: 5,
    };
    let mut privacy_key = [0u8; 32];
    privacy_key[..2].copy_from_slice(&operator_id.to_be_bytes());
    privacy_key[2..10].copy_from_slice(&epoch.to_be_bytes());
    privacy_key[10..].fill(0xa5);
    let mut controller = AdmissionController::new(config, 1, privacy_key)
        .map_err(|error| format!("invalid configuration: {error:?}"))?;
    let mut issued = BTreeMap::<[u8; 32], (u8, u64)>::new();

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "READY TEST_ONLY {operator_id} {epoch}").map_err(io_error)?;
    stdout.flush().map_err(io_error)?;

    while let Some(line) = read_bounded_line(&mut input).map_err(io_error)? {
        let Ok(line) = line else {
            writeln!(stdout, "REJECT").map_err(io_error)?;
            stdout.flush().map_err(io_error)?;
            continue;
        };
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        let response = match handle_command(fields.as_slice(), &mut controller, &mut issued) {
            Ok(NodeCommand::Response(response)) => response,
            Ok(NodeCommand::Quit) => {
                writeln!(stdout, "BYE").map_err(io_error)?;
                stdout.flush().map_err(io_error)?;
                return Ok(());
            }
            Err(()) => "REJECT".to_owned(),
        };
        writeln!(stdout, "{response}").map_err(io_error)?;
        stdout.flush().map_err(io_error)?;
    }
    Ok(())
}

fn read_bounded_line<R: Read>(reader: &mut R) -> io::Result<Option<Result<String, ()>>> {
    let mut bytes = Vec::with_capacity(MAX_COMMAND_BYTES);
    let mut overlong = false;
    let mut saw_input = false;
    loop {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte)? {
            0 if !saw_input => return Ok(None),
            0 => break,
            _ => saw_input = true,
        }
        if byte[0] == b'\n' {
            break;
        }
        if byte[0] == b'\r' {
            continue;
        }
        if bytes.len() < MAX_COMMAND_BYTES {
            bytes.push(byte[0]);
        } else {
            overlong = true;
        }
    }
    if overlong {
        return Ok(Some(Err(())));
    }
    Ok(Some(String::from_utf8(bytes).map_err(|_| ())))
}

enum NodeCommand {
    Response(String),
    Quit,
}

fn handle_command(
    fields: &[&str],
    controller: &mut AdmissionController,
    issued: &mut BTreeMap<[u8; 32], (u8, u64)>,
) -> Result<NodeCommand, ()> {
    let response = match fields {
        ["ADMIT", now, connection, network] => {
            let now = parse_u64(now).map_err(|_| ())?;
            let connection = parse_u64(connection).map_err(|_| ())?;
            issued.retain(|_, (_, expires_at)| now <= *expires_at);
            match controller.admit(now, connection, network.as_bytes(), None) {
                AdmissionDecision::Allow => "ALLOW".to_owned(),
                AdmissionDecision::RejectUniform => "REJECT".to_owned(),
                AdmissionDecision::Challenge {
                    challenge_id,
                    difficulty,
                    expires_at,
                } => {
                    issued.insert(challenge_id, (difficulty, expires_at));
                    format!(
                        "CHALLENGE {} {difficulty} {expires_at}",
                        encode_hex(challenge_id)
                    )
                }
            }
        }
        ["PROVE", now, connection, network, challenge, nonce] => {
            let now = parse_u64(now).map_err(|_| ())?;
            let connection = parse_u64(connection).map_err(|_| ())?;
            let challenge_id = decode_hex_32(challenge).map_err(|_| ())?;
            let nonce = parse_u64(nonce).map_err(|_| ())?;
            let verified = issued
                .remove(&challenge_id)
                .is_some_and(|(difficulty, expires_at)| {
                    now <= expires_at && verify_test_pow(challenge_id, nonce, difficulty)
                });
            match controller.admit(
                now,
                connection,
                network.as_bytes(),
                Some(PuzzleProof {
                    challenge_id,
                    verified,
                }),
            ) {
                AdmissionDecision::Allow => "ALLOW".to_owned(),
                AdmissionDecision::Challenge { .. } | AdmissionDecision::RejectUniform => {
                    "REJECT".to_owned()
                }
            }
        }
        ["ROTATE", window, key_byte] => {
            let key_byte = key_byte.parse::<u8>().map_err(|_| ())?;
            controller
                .rotate_window(parse_u64(window).map_err(|_| ())?, [key_byte; 32])
                .map_err(|_| ())?;
            issued.clear();
            "ROTATED".to_owned()
        }
        ["STATS"] => format!(
            "STATS {} {} {} {}",
            controller.admitted_in_window(),
            controller.tracked_connections(),
            controller.tracked_networks(),
            controller.pending_challenges()
        ),
        ["QUIT"] => return Ok(NodeCommand::Quit),
        _ => return Err(()),
    };
    Ok(NodeCommand::Response(response))
}

fn parse_args() -> Result<(u16, u64), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [operator_flag, operator, epoch_flag, epoch] = args.as_slice() else {
        return Err("usage: --operator <u16> --epoch <u64>".to_owned());
    };
    if operator_flag != "--operator" || epoch_flag != "--epoch" {
        return Err("usage: --operator <u16> --epoch <u64>".to_owned());
    }
    Ok((
        operator
            .parse::<u16>()
            .map_err(|_| "invalid operator".to_owned())?,
        parse_u64(epoch)?,
    ))
}

fn verify_test_pow(challenge_id: [u8; 32], nonce: u64, difficulty: u8) -> bool {
    let mut hash = Sha256::new();
    hash.update(POW_DOMAIN);
    hash.update(challenge_id);
    hash.update(nonce.to_be_bytes());
    leading_zero_bits(&hash.finalize()) >= u32::from(difficulty)
}

fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut total = 0;
    for byte in bytes {
        total += byte.leading_zeros();
        if *byte != 0 {
            break;
        }
    }
    total
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|_| "invalid u64".to_owned())
}

fn encode_hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.is_ascii() {
        return Err("invalid challenge".to_owned());
    }
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = (hex_nibble(value.as_bytes()[offset])? << 4)
            | hex_nibble(value.as_bytes()[offset + 1])?;
    }
    Ok(output)
}

fn hex_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("invalid challenge".to_owned()),
    }
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
