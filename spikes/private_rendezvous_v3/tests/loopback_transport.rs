use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    read_frame, signed_request, verify_response, write_frame, AuthenticatedRequest,
    AuthenticatedResponse, TransportError, MAX_AUTHENTICATED_FRAME_BYTES,
};
use std::{io::BufRead, io::BufReader};
use std::{
    io::Write,
    net::{Shutdown, SocketAddr, TcpStream},
    process::{Child, ChildStdout, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CLIENT_TEST_SEED: [u8; 32] = [0xc1; 32];

struct LoopbackNode {
    child: Child,
    address: SocketAddr,
    operator_key: VerifyingKey,
    operator_id: u16,
    epoch: u64,
    _output: BufReader<ChildStdout>,
}

impl LoopbackNode {
    fn spawn(operator_id: u16) -> Self {
        let epoch = 11;
        let mut child = Command::new(env!("CARGO_BIN_EXE_v3_loopback_operator"))
            .args([
                "--operator",
                &operator_id.to_string(),
                "--epoch",
                &epoch.to_string(),
                "--timeout-ms",
                "200",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        output.read_line(&mut ready).unwrap();
        let fields = ready.split_ascii_whitespace().collect::<Vec<_>>();
        assert_eq!(
            &fields[..4],
            &["READY", "TEST_ONLY", &operator_id.to_string(), "11"]
        );
        let address = fields[4].parse().unwrap();
        let operator_key = VerifyingKey::from_bytes(&decode_hex_32(fields[5])).unwrap();
        Self {
            child,
            address,
            operator_key,
            operator_id,
            epoch,
            _output: output,
        }
    }

    fn exchange(&self, frame: &[u8], sequence: u64) -> AuthenticatedResponse {
        let mut stream = self.connect();
        write_frame(&mut stream, frame).unwrap();
        let response = read_frame(&mut stream).unwrap();
        verify_response(
            &response,
            &self.operator_key,
            self.operator_id,
            self.epoch,
            sequence,
        )
        .unwrap()
    }

    fn rejected(&self, frame: &[u8]) {
        let mut stream = self.connect();
        write_frame(&mut stream, frame).unwrap();
        assert!(matches!(
            read_frame(&mut stream),
            Err(TransportError::Io(_))
        ));
    }

    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        stream
    }

    fn stop(&mut self, sequence: u64) {
        let request = request(self.operator_id, self.epoch, sequence, now_ms(), b"QUIT");
        let response = self.exchange(&request, sequence);
        assert_eq!(response.status, 0);
        assert_eq!(response.payload, b"BYE");
        assert!(self.child.wait().unwrap().success());
    }
}

impl Drop for LoopbackNode {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn signed_loopback_rejects_replay_forgery_context_skew_oversize_and_idle_clients() {
    let mut node = LoopbackNode::spawn(7);
    let timestamp = now_ms();
    let ping = request(7, 11, 1, timestamp, b"PING");
    let response = node.exchange(&ping, 1);
    assert_eq!(response.status, 0);
    assert_eq!(response.payload, b"PONG");

    node.rejected(&ping);

    let mut forged = request(7, 11, 2, timestamp, b"PING");
    let last = forged.len() - 1;
    forged[last] ^= 1;
    node.rejected(&forged);

    let wrong_context = request(8, 11, 2, timestamp, b"PING");
    node.rejected(&wrong_context);

    let expired = request(7, 11, 2, timestamp.saturating_sub(10_000), b"PING");
    node.rejected(&expired);

    let mut oversized = node.connect();
    oversized
        .write_all(&((MAX_AUTHENTICATED_FRAME_BYTES + 1) as u32).to_be_bytes())
        .unwrap();
    oversized.shutdown(Shutdown::Write).unwrap();
    assert!(matches!(
        read_frame(&mut oversized),
        Err(TransportError::Io(_))
    ));

    let idle = node.connect();
    std::thread::sleep(Duration::from_millis(300));
    drop(idle);

    let second = request(7, 11, 2, now_ms(), b"STATS");
    let response = node.exchange(&second, 2);
    assert_eq!(response.status, 0);
    assert_eq!(response.payload, b"2");
    node.stop(3);
}

fn request(operator: u16, epoch: u64, sequence: u64, time: u64, command: &[u8]) -> Vec<u8> {
    signed_request(
        &SigningKey::from_bytes(&CLIENT_TEST_SEED),
        &AuthenticatedRequest {
            operator_id: operator,
            epoch,
            sequence,
            issued_at_ms: time,
            command: command.to_vec(),
        },
    )
    .unwrap()
}

fn now_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn decode_hex_32(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    let mut output = [0u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}
