use ed25519_dalek::{SigningKey, VerifyingKey};
use scotoma_private_rendezvous_v3_model::transport::{
    read_frame, signed_request, verify_response, write_frame, AuthenticatedRequest,
    AuthenticatedResponse,
};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    process::{Child, ChildStdout, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Coordinator {
    child: Child,
    address: SocketAddr,
    verifying_key: VerifyingKey,
    id: u16,
    epoch: u64,
    _output: BufReader<ChildStdout>,
}

impl Coordinator {
    fn spawn(id: u16) -> Self {
        let epoch = 8;
        let mut child = Command::new(env!("CARGO_BIN_EXE_v3_ceremony_coordinator"))
            .args([
                "--coordinator",
                &id.to_string(),
                "--epoch",
                &epoch.to_string(),
                "--timeout-ms",
                "500",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        output.read_line(&mut ready).unwrap();
        let fields = ready.split_ascii_whitespace().collect::<Vec<_>>();
        assert_eq!(fields[0], "READY");
        assert_eq!(fields[1], "TEST_ONLY");
        assert_eq!(fields[2], id.to_string());
        assert_eq!(fields[3], "8");
        Self {
            child,
            address: fields[4].parse().unwrap(),
            verifying_key: VerifyingKey::from_bytes(&decode_hex_32(fields[5])).unwrap(),
            id,
            epoch,
            _output: output,
        }
    }

    fn request(&self, participant: u16, sequence: u64, command: &str) -> AuthenticatedResponse {
        let frame = signed_request(
            &participant_signing_key(participant),
            &AuthenticatedRequest {
                operator_id: self.id,
                epoch: self.epoch,
                sequence,
                issued_at_ms: now_ms(),
                command: command.as_bytes().to_vec(),
            },
        )
        .unwrap();
        let mut stream = TcpStream::connect(self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        write_frame(&mut stream, &frame).unwrap();
        let response = read_frame(&mut stream).unwrap();
        verify_response(
            &response,
            &self.verifying_key,
            self.id,
            self.epoch,
            sequence,
        )
        .unwrap()
    }

    fn stop(&mut self, participant: u16, sequence: u64) {
        let response = self.request(participant, sequence, "QUIT");
        assert_eq!(response.payload, b"BYE");
        assert!(self.child.wait().unwrap().success());
    }

    fn crash(&mut self) {
        self.child.kill().unwrap();
        let _ = self.child.wait();
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn two_coordinators_reach_the_same_ready_rotation() {
    let mut first = Coordinator::spawn(50);
    let mut second = Coordinator::spawn(51);
    let mut final_reports = Vec::new();
    for coordinator in [&first, &second] {
        let reports = deliver(coordinator, &honest_events());
        final_reports.push(reports.last().unwrap().clone());
    }
    assert!(activation_quorum(&final_reports));
    assert!(!activation_quorum(&[
        final_reports[0].clone(),
        final_reports[0].clone()
    ]));
    first.stop(2, 3);
    second.stop(2, 3);
}

#[test]
fn partition_timeout_and_process_loss_never_form_an_activation_quorum() {
    let mut first = Coordinator::spawn(60);
    let mut second = Coordinator::spawn(61);
    let events = honest_events();
    let prefix = &events[..events.len() - 1];
    deliver(&first, prefix);
    deliver(&second, prefix);

    let (participant, sequence, command) = events.last().unwrap();
    let ready = first.request(*participant, *sequence, command);
    let timed_out = second.request(2, 2, "TEST_TIMEOUT");
    assert!(payload(&ready).contains("PHASE READY"));
    assert!(payload(&timed_out).contains("PHASE TERMINAL"));
    assert!(!activation_quorum(&[ready, timed_out]));
    first.stop(2, 3);
    second.stop(2, 3);

    let mut surviving = Coordinator::spawn(62);
    let mut crashed = Coordinator::spawn(63);
    deliver(&surviving, &honest_events());
    crashed.crash();
    let one_vote = surviving.request(2, 3, "STATUS");
    assert!(!activation_quorum(&[one_vote]));
    surviving.stop(2, 4);
}

#[test]
fn substantiated_complaint_aborts_before_ready() {
    let mut coordinator = Coordinator::spawn(70);
    let events = honest_events();
    deliver(&coordinator, &events[..8]);
    let complaint = coordinator.request(2, 1, "COMPLAIN 1 2 133 229");
    assert_eq!(complaint.status, 1);
    assert!(payload(&complaint).contains("PHASE TERMINAL"));
    assert!(payload(&complaint).contains("SubstantiatedComplaint"));
    assert!(!activation_quorum(&[complaint]));
    coordinator.stop(2, 2);
}

fn honest_events() -> Vec<(u16, u64, &'static str)> {
    vec![
        (0, 1, "COMMIT 1 33"),
        (1, 1, "COMMIT 2 34"),
        (0, 2, "SHARE 0 1 128"),
        (0, 3, "SHARE 1 1 129"),
        (0, 4, "SHARE 2 1 130"),
        (1, 2, "SHARE 0 2 131"),
        (1, 3, "SHARE 1 2 132"),
        (1, 4, "SHARE 2 2 133"),
        (0, 5, "VERIFY 0 1 128 224"),
        (0, 6, "VERIFY 1 2 131 227"),
        (1, 5, "VERIFY 0 1 129 225"),
        (1, 6, "VERIFY 1 2 132 228"),
        (2, 1, "VERIFY 0 1 130 226"),
        (2, 2, "VERIFY 1 2 133 229"),
    ]
}

fn deliver(
    coordinator: &Coordinator,
    events: &[(u16, u64, &'static str)],
) -> Vec<AuthenticatedResponse> {
    events
        .iter()
        .map(|(participant, sequence, command)| {
            let response = coordinator.request(*participant, *sequence, command);
            assert_eq!(response.status, 0, "{}", payload(&response));
            response
        })
        .collect()
}

fn activation_quorum(reports: &[AuthenticatedResponse]) -> bool {
    let Some(first) = reports.first() else {
        return false;
    };
    let mut coordinators = BTreeSet::new();
    reports.iter().all(|report| {
        coordinators.insert(report.operator_id)
            && report.status == 0
            && report.epoch == first.epoch
            && report.payload == first.payload
            && payload(report).contains("PHASE READY")
    }) && coordinators.len() >= 2
}

fn payload(response: &AuthenticatedResponse) -> String {
    String::from_utf8(response.payload.clone()).unwrap()
}

fn participant_signing_key(participant_id: u16) -> SigningKey {
    let mut seed = [0x71; 32];
    seed[..2].copy_from_slice(&participant_id.to_be_bytes());
    SigningKey::from_bytes(&seed)
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
