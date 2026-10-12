use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

const POW_DOMAIN: &[u8] = b"SCOTOMA-v3-test-only-pow";

struct Node {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Node {
    fn spawn(operator: u16) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_v3_operational_node"))
            .args(["--operator", &operator.to_string(), "--epoch", "11"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        output.read_line(&mut ready).unwrap();
        assert_eq!(ready.trim(), format!("READY TEST_ONLY {operator} 11"));
        Self {
            child,
            input,
            output,
        }
    }

    fn request(&mut self, request: &str) -> String {
        writeln!(self.input, "{request}").unwrap();
        self.input.flush().unwrap();
        let mut response = String::new();
        self.output.read_line(&mut response).unwrap();
        response.trim().to_owned()
    }

    fn stop(&mut self) {
        assert_eq!(self.request("QUIT"), "BYE");
        assert!(self.child.wait().unwrap().success());
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn two_processes_isolate_challenges_and_verify_bounded_work() {
    let mut first = Node::spawn(1);
    let mut second = Node::spawn(2);
    for node in [&mut first, &mut second] {
        assert_eq!(node.request("ADMIT 10 7 shared-net"), "ALLOW");
        assert_eq!(node.request("ADMIT 11 7 shared-net"), "ALLOW");
    }

    let first_challenge = parse_challenge(&first.request("ADMIT 12 7 shared-net"));
    let second_challenge = parse_challenge(&second.request("ADMIT 12 7 shared-net"));
    assert_ne!(first_challenge.0, second_challenge.0);

    let first_nonce = solve(first_challenge.0, first_challenge.1);
    assert_eq!(
        second.request(&format!(
            "PROVE 13 7 shared-net {} {first_nonce}",
            encode_hex(first_challenge.0)
        )),
        "REJECT"
    );

    let second_nonce = solve(second_challenge.0, second_challenge.1);
    assert_eq!(
        second.request(&format!(
            "PROVE 13 7 shared-net {} {second_nonce}",
            encode_hex(second_challenge.0)
        )),
        "ALLOW"
    );
    assert_eq!(
        first.request(&format!(
            "PROVE 13 7 shared-net {} {first_nonce}",
            encode_hex(first_challenge.0)
        )),
        "ALLOW"
    );
    assert_eq!(first.request("STATS"), "STATS 3 1 1 0");
    assert_eq!(second.request("STATS"), "STATS 3 1 1 0");
    first.stop();
    second.stop();
}

#[test]
fn restart_drops_ephemeral_state_and_global_capacity_remains_absolute() {
    let mut node = Node::spawn(3);
    assert_eq!(node.request("ADMIT invalid 1 old-net"), "REJECT");
    assert_eq!(node.request("PROVE 1 1 old-net not-hex invalid"), "REJECT");
    assert_eq!(node.request(&"X".repeat(400)), "REJECT");
    assert_eq!(node.request("UNKNOWN"), "REJECT");
    assert_eq!(node.request("STATS"), "STATS 0 0 0 0");
    assert_eq!(node.request("ADMIT 1 1 old-net"), "ALLOW");
    node.stop();

    let mut restarted = Node::spawn(3);
    assert_eq!(restarted.request("STATS"), "STATS 0 0 0 0");
    for index in 0..8 {
        assert_eq!(
            restarted.request(&format!("ADMIT 1 {index} network-{index}")),
            "ALLOW"
        );
    }
    assert_eq!(restarted.request("ADMIT 1 99 overflow-net"), "REJECT");
    assert_eq!(restarted.request("STATS"), "STATS 8 8 8 0");
    restarted.stop();
}

fn parse_challenge(response: &str) -> ([u8; 32], u8) {
    let fields = response.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0], "CHALLENGE");
    (decode_hex_32(fields[1]), fields[2].parse().unwrap())
}

fn solve(challenge_id: [u8; 32], difficulty: u8) -> u64 {
    for nonce in 0..1_000_000_u64 {
        let mut hash = Sha256::new();
        hash.update(POW_DOMAIN);
        hash.update(challenge_id);
        hash.update(nonce.to_be_bytes());
        if leading_zero_bits(&hash.finalize()) >= u32::from(difficulty) {
            return nonce;
        }
    }
    panic!("bounded test puzzle had no solution");
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

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex_32(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
