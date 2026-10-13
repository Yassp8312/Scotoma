use ed25519_dalek::SigningKey;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
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
            "scotoma-v3-http-probe-{}-{suffix}.seed",
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

struct HttpOperator {
    child: Child,
    output: BufReader<ChildStdout>,
    address: SocketAddr,
    public_key: String,
}

impl HttpOperator {
    fn spawn(seed: &SeedFile, client_key: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_v3_public_probe_http_operator"))
            .args([
                "--operator",
                "303",
                "--epoch",
                "51",
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
        assert_eq!(fields.len(), 7);
        assert_eq!(&fields[..3], &["READY", "RESEARCH_ONLY", "HTTP"]);
        Self {
            child,
            output,
            address: fields[5].parse().unwrap(),
            public_key: fields[6].to_owned(),
        }
    }

    fn request(&self, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(self.address).unwrap();
        write!(
            stream,
            "POST /probe HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers
            .split_ascii_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, body.to_owned())
    }
}

impl Drop for HttpOperator {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut trailing = String::new();
        let _ = self.output.read_line(&mut trailing);
    }
}

#[test]
fn http_probe_preserves_signed_context_and_rejects_replay() {
    let client_seed = SeedFile::new(0xc3);
    let operator_seed = SeedFile::new(0x33);
    let client_public = encode_hex(
        SigningKey::from_bytes(&[0xc3; 32])
            .verifying_key()
            .to_bytes(),
    );
    let operator = HttpOperator::spawn(&operator_seed, &client_public);
    let request = make_request(&client_seed, 1);
    let (status, response) = operator.request(request.trim());
    assert_eq!(status, 200);
    let verified = verify_response(&operator.public_key, 1, &response);
    assert!(verified.status.success());
    let stdout = String::from_utf8_lossy(&verified.stdout);
    assert!(stdout.contains("\"result\":\"pass\""));
    assert!(stdout.contains("\"accepted\":1"));
    assert!(stdout.contains("\"receipt_sha256\":"));

    let (replay_status, replay_body) = operator.request(request.trim());
    assert_eq!(replay_status, 400);
    assert_eq!(replay_body, "invalid probe");
}

fn make_request(client_seed: &SeedFile, sequence: u64) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_v3_public_probe_http_client"))
        .args([
            "request",
            "--operator",
            "303",
            "--epoch",
            "51",
            "--sequence",
            &sequence.to_string(),
        ])
        .env("SCOTOMA_V3_PROBE_CLIENT_SEED_FILE", &client_seed.path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

fn verify_response(operator_key: &str, sequence: u64, response: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_v3_public_probe_http_client"))
        .args([
            "verify",
            "--operator",
            "303",
            "--epoch",
            "51",
            "--sequence",
            &sequence.to_string(),
            "--operator-key",
            operator_key,
            "--response-hex",
            response,
        ])
        .output()
        .unwrap()
}

fn encode_hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
