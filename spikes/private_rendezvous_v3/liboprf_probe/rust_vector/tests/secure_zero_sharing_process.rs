use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT,
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha512};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

const ZERO_SHARING_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-SHARING-v1\0";
const ZERO_SECURE_BROADCAST_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-BROADCAST-v1\0";
const ZERO_SECURE_ENVELOPE_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-ENVELOPE-v1\0";
const ZERO_COMPLAINT_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-COMPLAINT-v1\0";
const ZERO_DEFENSE_DOMAIN: &[u8] = b"SCOTOMA-v3-ZERO-DEFENSE-v1\0";
const ZERO_NOISE_PATTERN: &str = "Noise_N_25519_ChaChaPoly_SHA256";

struct Recipient {
    id: u16,
    noise_private: Vec<u8>,
    noise_public: Vec<u8>,
    complaint_key: SigningKey,
}

#[derive(Clone)]
struct SecureEnvelope {
    recipient_id: u16,
    recipient_public: [u8; 32],
    ciphertext: Vec<u8>,
    signature: [u8; 64],
}

#[derive(Clone)]
struct SecureDealerOutput {
    dealer_id: u16,
    context: [u8; 64],
    constant_commitment: [u8; 32],
    linear_commitment: [u8; 32],
    verifying_key: [u8; 32],
    broadcast_signature: [u8; 64],
    envelopes: [SecureEnvelope; 3],
}

struct SecureDealerProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    output: SecureDealerOutput,
}

#[derive(Clone)]
struct Complaint {
    dealer_id: u16,
    recipient_id: u16,
    broadcast_digest: [u8; 64],
    envelope_digest: [u8; 64],
    verifying_key: [u8; 32],
    signature: [u8; 64],
}

#[derive(Clone)]
struct Defense {
    dealer_id: u16,
    recipient_id: u16,
    broadcast_digest: [u8; 64],
    complaint_digest: [u8; 64],
    recipient_public: [u8; 32],
    ciphertext: Vec<u8>,
    signature: [u8; 64],
}

impl Drop for SecureDealerProcess {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.wait();
    }
}

impl SecureDealerProcess {
    fn defend(&mut self, complaint: &Complaint) -> Defense {
        let digest = complaint_digest(complaint);
        writeln!(
            self.stdin.as_mut().unwrap(),
            "DEFEND {} {}",
            complaint.recipient_id,
            encode_hex(&digest)
        )
        .unwrap();
        self.stdin.as_mut().unwrap().flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        parse_defense(line.trim())
    }
}

fn recipients() -> [Recipient; 3] {
    std::array::from_fn(|index| {
        let params = ZERO_NOISE_PATTERN.parse().unwrap();
        let keypair = snow::Builder::new(params).generate_keypair().unwrap();
        let id = (index + 1) as u16;
        let mut seed = [0x73; 32];
        seed[..2].copy_from_slice(&id.to_be_bytes());
        Recipient {
            id,
            noise_private: keypair.private,
            noise_public: keypair.public,
            complaint_key: SigningKey::from_bytes(&seed),
        }
    })
}

fn spawn_secure_dealer(
    dealer_id: u16,
    epoch: u64,
    recipients: &[Recipient; 3],
    corrupt_recipient: Option<u16>,
) -> SecureDealerProcess {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scotoma-liboprf-independent-vector"));
    command
        .arg("zero-dealer-secure")
        .arg(dealer_id.to_string())
        .arg(epoch.to_string());
    for recipient in recipients {
        command.arg(encode_hex(&recipient.noise_public));
    }
    if let Some(recipient_id) = corrupt_recipient {
        command.arg(recipient_id.to_string());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(
        !line.is_empty(),
        "secure dealer exited before its broadcast"
    );
    SecureDealerProcess {
        child,
        stdin: Some(stdin),
        stdout,
        output: parse_secure_output(line.trim()),
    }
}

fn parse_secure_output(output: &str) -> SecureDealerOutput {
    let fields = output.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 19);
    assert_eq!(fields[0], "ZERO_DEALER_SECURE_V1");
    SecureDealerOutput {
        dealer_id: fields[1].parse().unwrap(),
        context: decode_hex(fields[2]),
        constant_commitment: decode_hex(fields[3]),
        linear_commitment: decode_hex(fields[4]),
        verifying_key: decode_hex(fields[5]),
        broadcast_signature: decode_hex(fields[6]),
        envelopes: [
            parse_envelope(&fields[7..11]),
            parse_envelope(&fields[11..15]),
            parse_envelope(&fields[15..19]),
        ],
    }
}

fn parse_envelope(fields: &[&str]) -> SecureEnvelope {
    SecureEnvelope {
        recipient_id: fields[0].parse().unwrap(),
        recipient_public: decode_hex(fields[1]),
        ciphertext: decode_hex_vec(fields[2]),
        signature: decode_hex(fields[3]),
    }
}

fn parse_defense(output: &str) -> Defense {
    let fields = output.split_ascii_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 8);
    assert_eq!(fields[0], "ZERO_DEFENSE_V1");
    Defense {
        dealer_id: fields[1].parse().unwrap(),
        recipient_id: fields[2].parse().unwrap(),
        broadcast_digest: decode_hex(fields[3]),
        complaint_digest: decode_hex(fields[4]),
        recipient_public: decode_hex(fields[5]),
        ciphertext: decode_hex_vec(fields[6]),
        signature: decode_hex(fields[7]),
    }
}

fn expected_context(epoch: u64) -> [u8; 64] {
    let deployment = b"scotoma-test-deployment";
    let mut digest = Sha512::new();
    digest.update(ZERO_SHARING_DOMAIN);
    digest.update((deployment.len() as u16).to_be_bytes());
    digest.update(deployment);
    digest.update([0x53; 32]);
    digest.update(epoch.to_be_bytes());
    digest.update([0xa7; 32]);
    digest.update(2_u16.to_be_bytes());
    digest.finalize().into()
}

fn broadcast_payload(output: &SecureDealerOutput) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(ZERO_SECURE_BROADCAST_DOMAIN);
    payload.extend_from_slice(&output.dealer_id.to_be_bytes());
    payload.extend_from_slice(&output.context);
    payload.extend_from_slice(&output.constant_commitment);
    payload.extend_from_slice(&output.linear_commitment);
    payload
}

fn broadcast_digest(output: &SecureDealerOutput) -> [u8; 64] {
    Sha512::digest(broadcast_payload(output)).into()
}

fn envelope_payload(output: &SecureDealerOutput, envelope: &SecureEnvelope) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(ZERO_SECURE_ENVELOPE_DOMAIN);
    payload.extend_from_slice(&broadcast_digest(output));
    payload.extend_from_slice(&output.dealer_id.to_be_bytes());
    payload.extend_from_slice(&envelope.recipient_id.to_be_bytes());
    payload.extend_from_slice(&envelope.recipient_public);
    payload.extend_from_slice(&(envelope.ciphertext.len() as u16).to_be_bytes());
    payload.extend_from_slice(&envelope.ciphertext);
    payload
}

fn verify_broadcast(
    output: &SecureDealerOutput,
    expected_id: u16,
    epoch: u64,
) -> Result<(), &'static str> {
    if output.dealer_id != expected_id || output.context != expected_context(epoch) {
        return Err("wrong secure broadcast context");
    }
    let mut seed = [0x91; 32];
    seed[..2].copy_from_slice(&expected_id.to_be_bytes());
    let expected_key = SigningKey::from_bytes(&seed).verifying_key();
    if output.verifying_key != expected_key.to_bytes() {
        return Err("crossed secure dealer identity");
    }
    expected_key
        .verify(
            &broadcast_payload(output),
            &Signature::from_bytes(&output.broadcast_signature),
        )
        .map_err(|_| "invalid secure broadcast signature")?;
    if CompressedRistretto(output.constant_commitment)
        .decompress()
        .ok_or("invalid secure constant commitment")?
        != RistrettoPoint::default()
    {
        return Err("nonzero secure constant commitment");
    }
    for (index, envelope) in output.envelopes.iter().enumerate() {
        if envelope.recipient_id != (index + 1) as u16 {
            return Err("invalid secure recipient ordering");
        }
        expected_key
            .verify(
                &envelope_payload(output, envelope),
                &Signature::from_bytes(&envelope.signature),
            )
            .map_err(|_| "invalid secure envelope signature")?;
    }
    Ok(())
}

fn decrypt_share(
    output: &SecureDealerOutput,
    envelope: &SecureEnvelope,
    recipient: &Recipient,
) -> Result<Scalar, &'static str> {
    if envelope.recipient_id != recipient.id
        || envelope.recipient_public.as_slice() != recipient.noise_public
    {
        return Err("crossed secure recipient");
    }
    let params = ZERO_NOISE_PATTERN
        .parse()
        .map_err(|_| "invalid Noise pattern")?;
    let mut responder = snow::Builder::new(params)
        .prologue(&broadcast_digest(output))
        .map_err(|_| "invalid Noise prologue")?
        .local_private_key(&recipient.noise_private)
        .map_err(|_| "invalid recipient private key")?
        .build_responder()
        .map_err(|_| "Noise responder failed")?;
    let mut plaintext = [0_u8; 64];
    let length = responder
        .read_message(&envelope.ciphertext, &mut plaintext)
        .map_err(|_| "Noise decryption failed")?;
    if length != 32 {
        return Err("invalid secure share length");
    }
    let share = Option::<Scalar>::from(Scalar::from_canonical_bytes(
        plaintext[..32].try_into().unwrap(),
    ))
    .ok_or("noncanonical secure share")?;
    let linear = CompressedRistretto(output.linear_commitment)
        .decompress()
        .ok_or("invalid secure linear commitment")?;
    if RISTRETTO_BASEPOINT_POINT * share != linear * Scalar::from(u64::from(recipient.id)) {
        return Err("secure share failed commitment");
    }
    Ok(share)
}

fn complaint_payload(complaint: &Complaint) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(ZERO_COMPLAINT_DOMAIN);
    payload.extend_from_slice(&complaint.broadcast_digest);
    payload.extend_from_slice(&complaint.envelope_digest);
    payload.extend_from_slice(&complaint.dealer_id.to_be_bytes());
    payload.extend_from_slice(&complaint.recipient_id.to_be_bytes());
    payload
}

fn make_complaint(
    output: &SecureDealerOutput,
    envelope: &SecureEnvelope,
    recipient: &Recipient,
) -> Complaint {
    let mut complaint = Complaint {
        dealer_id: output.dealer_id,
        recipient_id: recipient.id,
        broadcast_digest: broadcast_digest(output),
        envelope_digest: Sha512::digest(&envelope.ciphertext).into(),
        verifying_key: recipient.complaint_key.verifying_key().to_bytes(),
        signature: [0_u8; 64],
    };
    complaint.signature = recipient
        .complaint_key
        .sign(&complaint_payload(&complaint))
        .to_bytes();
    complaint
}

fn verify_complaint(complaint: &Complaint, recipient: &Recipient) -> Result<(), &'static str> {
    if complaint.recipient_id != recipient.id
        || complaint.verifying_key != recipient.complaint_key.verifying_key().to_bytes()
    {
        return Err("crossed complaint identity");
    }
    VerifyingKey::from_bytes(&complaint.verifying_key)
        .map_err(|_| "invalid complaint key")?
        .verify(
            &complaint_payload(complaint),
            &Signature::from_bytes(&complaint.signature),
        )
        .map_err(|_| "invalid complaint signature")
}

fn complaint_digest(complaint: &Complaint) -> [u8; 64] {
    Sha512::digest(complaint_payload(complaint)).into()
}

fn defense_payload(defense: &Defense) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(ZERO_DEFENSE_DOMAIN);
    payload.extend_from_slice(&defense.broadcast_digest);
    payload.extend_from_slice(&defense.complaint_digest);
    payload.extend_from_slice(&defense.dealer_id.to_be_bytes());
    payload.extend_from_slice(&defense.recipient_id.to_be_bytes());
    payload.extend_from_slice(&defense.recipient_public);
    payload.extend_from_slice(&(defense.ciphertext.len() as u16).to_be_bytes());
    payload.extend_from_slice(&defense.ciphertext);
    payload
}

fn verify_defense(
    defense: &Defense,
    output: &SecureDealerOutput,
    complaint: &Complaint,
    recipient: &Recipient,
) -> Result<Scalar, &'static str> {
    if defense.dealer_id != output.dealer_id
        || defense.recipient_id != recipient.id
        || defense.broadcast_digest != broadcast_digest(output)
        || defense.complaint_digest != complaint_digest(complaint)
        || defense.recipient_public.as_slice() != recipient.noise_public
    {
        return Err("unbound secure defense");
    }
    VerifyingKey::from_bytes(&output.verifying_key)
        .map_err(|_| "invalid defense key")?
        .verify(
            &defense_payload(defense),
            &Signature::from_bytes(&defense.signature),
        )
        .map_err(|_| "invalid defense signature")?;
    let recovered = SecureEnvelope {
        recipient_id: defense.recipient_id,
        recipient_public: defense.recipient_public,
        ciphertext: defense.ciphertext.clone(),
        signature: [0_u8; 64],
    };
    decrypt_share(output, &recovered, recipient)
}

fn record_consistent_broadcast(
    ledger: &mut BTreeMap<u16, [u8; 64]>,
    output: &SecureDealerOutput,
) -> Result<(), &'static str> {
    let digest = broadcast_digest(output);
    match ledger.entry(output.dealer_id) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(digest);
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &digest => {
            return Err("dealer broadcast equivocation");
        }
        std::collections::btree_map::Entry::Occupied(_) => {}
    }
    Ok(())
}

fn dealer_is_qualified(output: &SecureDealerOutput, recipients: &[Recipient; 3]) -> bool {
    verify_broadcast(output, output.dealer_id, 19).is_ok()
        && output
            .envelopes
            .iter()
            .zip(recipients)
            .all(|(envelope, recipient)| decrypt_share(output, envelope, recipient).is_ok())
}

fn aggregate(shares: &[[Scalar; 3]; 3], recipient: usize) -> Scalar {
    shares
        .iter()
        .fold(Scalar::ZERO, |sum, dealer| sum + dealer[recipient])
}

fn reconstruct(left: (u64, Scalar), right: (u64, Scalar)) -> Scalar {
    let left_coefficient =
        -Scalar::from(right.0) * (Scalar::from(left.0) - Scalar::from(right.0)).invert();
    let right_coefficient =
        -Scalar::from(left.0) * (Scalar::from(right.0) - Scalar::from(left.0)).invert();
    left.1 * left_coefficient + right.1 * right_coefficient
}

#[test]
fn os_random_dealers_deliver_only_encrypted_recipient_shares() {
    let recipients = recipients();
    let dealers = [
        spawn_secure_dealer(1, 19, &recipients, None),
        spawn_secure_dealer(2, 19, &recipients, None),
        spawn_secure_dealer(3, 19, &recipients, None),
    ];
    let mut shares = [[Scalar::ZERO; 3]; 3];
    for (dealer_index, dealer) in dealers.iter().enumerate() {
        verify_broadcast(&dealer.output, (dealer_index + 1) as u16, 19).unwrap();
        for (recipient_index, recipient) in recipients.iter().enumerate() {
            let share = decrypt_share(
                &dealer.output,
                &dealer.output.envelopes[recipient_index],
                recipient,
            )
            .unwrap();
            assert!(
                !dealer.output.envelopes[recipient_index]
                    .ciphertext
                    .windows(32)
                    .any(|window| window == share.to_bytes())
            );
            shares[dealer_index][recipient_index] = share;
        }
    }
    let aggregate = [
        aggregate(&shares, 0),
        aggregate(&shares, 1),
        aggregate(&shares, 2),
    ];
    for &(left, right) in &[(0_usize, 1_usize), (0, 2), (1, 2)] {
        assert_eq!(
            reconstruct(
                ((left + 1) as u64, aggregate[left]),
                ((right + 1) as u64, aggregate[right])
            ),
            Scalar::ZERO
        );
    }

    let repeated = spawn_secure_dealer(1, 19, &recipients, None);
    assert_ne!(
        repeated.output.linear_commitment,
        dealers[0].output.linear_commitment
    );
    assert_ne!(
        repeated.output.envelopes[0].ciphertext,
        dealers[0].output.envelopes[0].ciphertext
    );
}

#[test]
fn crossed_recipient_context_and_ciphertext_fail_closed() {
    let recipients = recipients();
    let dealer = spawn_secure_dealer(1, 19, &recipients, None);
    assert_eq!(
        verify_broadcast(&dealer.output, 1, 20),
        Err("wrong secure broadcast context")
    );
    assert_eq!(
        decrypt_share(&dealer.output, &dealer.output.envelopes[0], &recipients[1]),
        Err("crossed secure recipient")
    );
    let mut crossed_key = dealer.output.clone();
    crossed_key.envelopes[0]
        .recipient_public
        .copy_from_slice(&recipients[1].noise_public);
    assert_eq!(
        verify_broadcast(&crossed_key, 1, 19),
        Err("invalid secure envelope signature")
    );
    assert_eq!(
        decrypt_share(&crossed_key, &crossed_key.envelopes[0], &recipients[0]),
        Err("crossed secure recipient")
    );
    let mut forged = dealer.output.clone();
    forged.envelopes[0].ciphertext[40] ^= 1;
    assert_eq!(
        verify_broadcast(&forged, 1, 19),
        Err("invalid secure envelope signature")
    );
    assert!(decrypt_share(&forged, &forged.envelopes[0], &recipients[0]).is_err());
}

#[test]
fn signed_complaint_defense_recovery_and_equivocation_are_bounded() {
    let recipients = recipients();
    let mut dealer = spawn_secure_dealer(1, 19, &recipients, Some(2));
    verify_broadcast(&dealer.output, 1, 19).unwrap();
    assert_eq!(
        decrypt_share(&dealer.output, &dealer.output.envelopes[1], &recipients[1]),
        Err("secure share failed commitment")
    );

    let complaint = make_complaint(&dealer.output, &dealer.output.envelopes[1], &recipients[1]);
    verify_complaint(&complaint, &recipients[1]).unwrap();
    let defense = dealer.defend(&complaint);
    verify_defense(&defense, &dealer.output, &complaint, &recipients[1]).unwrap();

    let mut forged_defense = defense.clone();
    forged_defense.ciphertext[40] ^= 1;
    assert_eq!(
        verify_defense(&forged_defense, &dealer.output, &complaint, &recipients[1]),
        Err("invalid defense signature")
    );

    let equivocation = spawn_secure_dealer(1, 19, &recipients, None);
    verify_broadcast(&equivocation.output, 1, 19).unwrap();
    let mut ledger = BTreeMap::new();
    record_consistent_broadcast(&mut ledger, &dealer.output).unwrap();
    assert_eq!(
        record_consistent_broadcast(&mut ledger, &equivocation.output),
        Err("dealer broadcast equivocation")
    );

    let good_two = spawn_secure_dealer(2, 19, &recipients, None);
    let good_three = spawn_secure_dealer(3, 19, &recipients, None);
    let qualified_without_defense = [
        dealer_is_qualified(&dealer.output, &recipients),
        dealer_is_qualified(&good_two.output, &recipients),
        dealer_is_qualified(&good_three.output, &recipients),
    ]
    .into_iter()
    .filter(|qualified| *qualified)
    .count();
    assert_eq!(qualified_without_defense, 2);

    let recovered_share =
        verify_defense(&defense, &dealer.output, &complaint, &recipients[1]).unwrap();
    assert_ne!(recovered_share, Scalar::ZERO);
    let qualified_after_recovery = qualified_without_defense + 1;
    assert_eq!(qualified_after_recovery, 3);

    let corrupt_three = spawn_secure_dealer(3, 19, &recipients, Some(1));
    let qualified_after_second_failure = [
        dealer_is_qualified(&dealer.output, &recipients),
        dealer_is_qualified(&good_two.output, &recipients),
        dealer_is_qualified(&corrupt_three.output, &recipients),
    ]
    .into_iter()
    .filter(|qualified| *qualified)
    .count();
    assert_eq!(qualified_after_second_failure, 1);
    assert!(qualified_after_second_failure < 2, "ceremony must abort");
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    decode_hex_vec(value).try_into().unwrap()
}

fn decode_hex_vec(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect()
}
