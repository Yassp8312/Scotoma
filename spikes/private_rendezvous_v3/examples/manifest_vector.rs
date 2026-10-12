use ed25519_dalek::SigningKey;
use scotoma_private_rendezvous_v3_model::manifest::{ManifestBody, Operator, SignedManifest};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() {
    let release = SigningKey::from_bytes(&[99; 32]);
    let operator_keys = [
        (10, SigningKey::from_bytes(&[10; 32])),
        (20, SigningKey::from_bytes(&[20; 32])),
        (30, SigningKey::from_bytes(&[30; 32])),
    ];
    let initial_body = ManifestBody {
        sequence: 1,
        minimum_client_version: 7,
        protocol_version: 3,
        suite_id: 1,
        construction_id: 1,
        threshold: 2,
        epoch: 41,
        valid_from: 100,
        valid_until: 200,
        previous_manifest_hash: [0; 32],
        operators: operator_keys
            .iter()
            .map(|(id, key)| Operator {
                id: *id,
                verifying_key: key.verifying_key().to_bytes(),
                endpoint: format!("https://operator-{id}.invalid"),
            })
            .collect(),
    };
    let initial = SignedManifest::new_for_research(initial_body, &release, &[]).unwrap();
    let initial_hash = initial.body.hash().unwrap();
    let mut update_body = initial.body.clone();
    update_body.sequence = 2;
    update_body.epoch = 42;
    update_body.previous_manifest_hash = initial_hash;
    let update = SignedManifest::new_for_research(
        update_body,
        &release,
        &[(10, &operator_keys[0].1), (20, &operator_keys[1].1)],
    )
    .unwrap();

    println!(
        "{{\n  \"schema\": \"scotoma-v3-signed-manifest-vector-v1\",\n  \"warning\": \"TEST-ONLY deterministic keys; never use these bytes in production\",\n  \"release_public_key\": \"{}\",\n  \"initial_manifest_hash\": \"{}\",\n  \"initial_envelope\": \"{}\",\n  \"update_manifest_hash\": \"{}\",\n  \"update_envelope\": \"{}\"\n}}",
        hex(&release.verifying_key().to_bytes()),
        hex(&initial_hash),
        hex(&initial.canonical_bytes().unwrap()),
        hex(&update.body.hash().unwrap()),
        hex(&update.canonical_bytes().unwrap()),
    );
}
