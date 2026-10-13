use ed25519_dalek::SigningKey;
use std::{
    fs,
    io::{BufRead, BufReader},
    net::SocketAddr,
    path::PathBuf,
    process::{Child, ChildStdout, Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FILE: AtomicU64 = AtomicU64::new(1);

struct SeedFile {
    path: PathBuf,
}

impl SeedFile {
    fn new(byte: u8) -> Self {
        let suffix = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "scotoma-v3-probe-{}-{suffix}.seed",
            std::process::id()
        ));
        fs::write(&path, encode_hex([byte; 32])).unwrap();
        Self { path }
    }
}

impl Drop for SeedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct Operator {
    child: Child,
    output: BufReader<ChildStdout>,
    address: SocketAddr,
    public_key: String,
    operator_id: u16,
    epoch: u64,
}

impl Operator {
    fn spawn(operator_id: u16, seed: &SeedFile, client_key: &str) -> Self {
        let epoch = 41;
        let mut child = Command::new(env!("CARGO_BIN_EXE_v3_public_probe_operator"))
            .args([
                "--operator",
                &operator_id.to_string(),
                "--epoch",
                &epoch.to_string(),
                "--bind",
                "127.0.0.1:0",
                "--client-key",
                client_key,
                "--timeout-ms",
                "500",
            ])
            .env("SCOTOMA_V3_OPERATOR_SEED_FILE", &seed.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        output.read_line(&mut ready).unwrap();
        let fields = ready.split_ascii_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 6);
        assert_eq!(&fields[..2], &["READY", "RESEARCH_ONLY"]);
        assert_eq!(fields[2], operator_id.to_string());
        assert_eq!(fields[3], epoch.to_string());
        Self {
            child,
            output,
            address: fields[4].parse().unwrap(),
            public_key: fields[5].to_owned(),
            operator_id,
            epoch,
        }
    }

    fn probe(&self, client_seed: &SeedFile, sequence: u64, public_key: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_v3_public_probe_client"))
            .args([
                "--operator",
                &self.operator_id.to_string(),
                "--epoch",
                &self.epoch.to_string(),
                "--endpoint",
                &self.address.to_string(),
                "--operator-key",
                public_key,
                "--sequence",
                &sequence.to_string(),
                "--timeout-ms",
                "500",
            ])
            .env("SCOTOMA_V3_PROBE_CLIENT_SEED_FILE", &client_seed.path)
            .output()
            .unwrap()
    }

    fn crash(&mut self) {
        self.child.kill().unwrap();
        let _ = self.child.wait();
    }
}

impl Drop for Operator {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut trailing = String::new();
        let _ = self.output.read_line(&mut trailing);
    }
}

#[test]
fn independent_identities_survive_single_failure_and_recover_without_shared_state() {
    let client_seed = SeedFile::new(0xc1);
    let first_seed = SeedFile::new(0x11);
    let second_seed = SeedFile::new(0x22);
    let client_public = encode_hex(
        SigningKey::from_bytes(&[0xc1; 32])
            .verifying_key()
            .to_bytes(),
    );
    let mut first = Operator::spawn(101, &first_seed, &client_public);
    let mut second = Operator::spawn(202, &second_seed, &client_public);

    assert_ne!(first.public_key, second.public_key);
    assert_pass(&first.probe(&client_seed, 1, &first.public_key), 101, 1);
    assert_pass(&second.probe(&client_seed, 1, &second.public_key), 202, 1);

    let wrong_identity = second.probe(&client_seed, 2, &first.public_key);
    assert!(!wrong_identity.status.success());
    assert!(String::from_utf8_lossy(&wrong_identity.stderr).contains("invalid signed receipt"));

    second.crash();
    assert_pass(&first.probe(&client_seed, 2, &first.public_key), 101, 2);
    let partitioned = second.probe(&client_seed, 3, &second.public_key);
    assert!(!partitioned.status.success());
    assert!(String::from_utf8_lossy(&partitioned.stderr).contains("operator unavailable"));

    let recovered = Operator::spawn(202, &second_seed, &client_public);
    assert_eq!(recovered.public_key, second.public_key);
    assert_pass(
        &recovered.probe(&client_seed, 1, &recovered.public_key),
        202,
        1,
    );

    first.crash();
}

fn assert_pass(output: &Output, operator_id: u16, accepted: u64) {
    assert!(
        output.status.success(),
        "probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"schema\":\"scotoma-v3-public-probe-v1\""));
    assert!(stdout.contains(&format!("\"operator\":{operator_id}")));
    assert!(stdout.contains(&format!("\"accepted\":{accepted}")));
    assert!(stdout.contains("\"result\":\"pass\""));
}

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
