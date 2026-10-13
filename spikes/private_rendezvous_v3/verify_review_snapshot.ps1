param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$snapshotPath = Join-Path $repositoryRoot 'docs/review/V3_REVIEW_SNAPSHOT.json'
$ledgerPath = Join-Path $repositoryRoot 'docs/review/V3_FINDINGS_LEDGER.json'
$snapshot = Get-Content -LiteralPath $snapshotPath -Raw | ConvertFrom-Json
$ledger = Get-Content -LiteralPath $ledgerPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()

if ($snapshot.schema -ne 'scotoma-v3-review-snapshot-v2') {
    $errors.Add('unexpected review snapshot schema')
}
$expectedPaths = @(
    '.github/workflows/ci.yml',
    '.gitignore',
    'docs/SUPPLY_CHAIN.md',
    'docs/PRIVATE_RENDEZVOUS_REQUIREMENTS.md',
    'docs/PRIVATE_RENDEZVOUS_V3_CANDIDATE.md',
    'docs/TOPRF_LIBRARY_EVALUATION.md',
    'docs/V3_TOPRF_NOISE_COMPOSITION.md',
    'docs/V3_OPERATOR_AND_ABUSE_MODEL.md',
    'docs/V3_MANIFEST_AND_COLLISION_POLICY.md',
    'docs/INTERNAL_V3_SECURITY_REVIEW.md',
    'docs/PRIVATE_RENDEZVOUS_V3_TEST_PLAN.md',
    'docs/V3_OPERATIONAL_BASELINE.md',
    'docs/adr/0061-canonical-signed-v3-manifest.md',
    'docs/adr/0062-malicious-relay-collision-limit.md',
    'docs/adr/0063-v3-supply-chain-gate.md',
    'docs/adr/0064-select-liboprf-3hashtdh-candidate.md',
    'docs/adr/0065-probe-real-liboprf-ceremony.md',
    'docs/adr/0066-cross-platform-3hashtdh-probe.md',
    'docs/adr/0067-independent-public-operator-probe.md',
    'docs/adr/0068-freeze-cocktail-dkg-specification-candidate.md',
    'docs/review/V3_CRYPTO_SELECTION.json',
    'docs/review/V3_CEREMONY_PROBE.json',
    'docs/review/V3_DKG_SELECTION.json',
    'docs/review/V3_PLATFORM_PROBE.json',
    'docs/review/V3_INFRASTRUCTURE_PROBE.json',
    'spikes/private_rendezvous_v3/Cargo.toml',
    'spikes/private_rendezvous_v3/Cargo.lock',
    'spikes/private_rendezvous_v3/README.md',
    'spikes/private_rendezvous_v3/fuzz/Cargo.toml',
    'spikes/private_rendezvous_v3/fuzz/Cargo.lock',
    'spikes/private_rendezvous_v3/fuzz/README.md',
    'spikes/private_rendezvous_v3/fuzz/fuzz_targets/v3_state_relay.rs',
    'spikes/private_rendezvous_v3/fuzz/fuzz_targets/v3_credential_noise.rs',
    'spikes/private_rendezvous_v3/fuzz/fuzz_targets/v3_manifest_epoch.rs',
    'spikes/private_rendezvous_v3/fuzz/fuzz_targets/v3_adversarial_schedule.rs',
    'spikes/private_rendezvous_v3/fuzz/run_linux_smoke.sh',
    'spikes/private_rendezvous_v3/src/lib.rs',
    'spikes/private_rendezvous_v3/src/ceremony.rs',
    'spikes/private_rendezvous_v3/src/epoch.rs',
    'spikes/private_rendezvous_v3/src/manifest.rs',
    'spikes/private_rendezvous_v3/src/operational.rs',
    'spikes/private_rendezvous_v3/src/relay.rs',
    'spikes/private_rendezvous_v3/src/transport.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_ceremony_coordinator.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_adversarial_relay.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_loopback_operator.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_operational_node.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_public_probe_client.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_public_probe_http_client.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_public_probe_http_operator.rs',
    'spikes/private_rendezvous_v3/src/bin/v3_public_probe_operator.rs',
    'spikes/private_rendezvous_v3/tests/hardening.rs',
    'spikes/private_rendezvous_v3/tests/adversarial_relay_process.rs',
    'spikes/private_rendezvous_v3/tests/loopback_transport.rs',
    'spikes/private_rendezvous_v3/tests/multiprocess_ceremony.rs',
    'spikes/private_rendezvous_v3/tests/multiprocess_operations.rs',
    'spikes/private_rendezvous_v3/tests/independent_operator_probe.rs',
    'spikes/private_rendezvous_v3/tests/http_operator_probe.rs',
    'spikes/private_rendezvous_v3/examples/operational_benchmark.rs',
    'spikes/private_rendezvous_v3/examples/manifest_vector.rs',
    'spikes/private_rendezvous_v3/examples/admission_simulation.rs',
    'spikes/private_rendezvous_v3/run_operational_benchmark.ps1',
    'spikes/private_rendezvous_v3/infrastructure/Dockerfile',
    'spikes/private_rendezvous_v3/infrastructure/Dockerfile.dockerignore',
    'spikes/private_rendezvous_v3/infrastructure/README.md',
    'spikes/private_rendezvous_v3/infrastructure/compose.local-simulation.yml',
    'spikes/private_rendezvous_v3/infrastructure/operator-set.example.json',
    'spikes/private_rendezvous_v3/infrastructure/operator-set-http.example.json',
    'spikes/private_rendezvous_v3/infrastructure/operator-set-http.test-only.json',
    'spikes/private_rendezvous_v3/infrastructure/operator-set.test-only.json',
    'spikes/private_rendezvous_v3/infrastructure/render.operator.example.yaml',
    'spikes/private_rendezvous_v3/infrastructure/run_public_http_probe.ps1',
    'spikes/private_rendezvous_v3/infrastructure/run_public_probe.ps1',
    'spikes/private_rendezvous_v3/infrastructure/verify_operator_set.ps1',
    'spikes/private_rendezvous_v3/liboprf_probe/README.md',
    'spikes/private_rendezvous_v3/liboprf_probe/scotoma_3hashtdh_probe.c',
    'spikes/private_rendezvous_v3/liboprf_probe/scotoma_3hashtdh_vector.c',
    'spikes/private_rendezvous_v3/liboprf_probe/run_linux_probe.sh',
    'spikes/private_rendezvous_v3/liboprf_probe/run_platform_builds.sh',
    'spikes/private_rendezvous_v3/liboprf_probe/verify_vector.ps1',
    'spikes/private_rendezvous_v3/liboprf_probe/vectors/3hashtdh_test_only_001.json',
    'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/Cargo.toml',
    'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/Cargo.lock',
    'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/src/main.rs',
    'spikes/private_rendezvous_v3/verify_crypto_selection.ps1',
    'spikes/private_rendezvous_v3/verify_dkg_sources.ps1',
    'spikes/private_rendezvous_v3/run_cocktail_dkg_probe.sh',
    'spikes/private_rendezvous_v3/verify_review_snapshot.ps1',
    'spikes/private_rendezvous_v3/verify_supply_chain.ps1',
    'spikes/private_rendezvous_v3/vectors/signed_manifest_v1.json',
    'spikes/private_rendezvous_v3/web/manifest_verifier.js',
    'spikes/private_rendezvous_v3/taceo_probe/Cargo.toml',
    'spikes/private_rendezvous_v3/taceo_probe/Cargo.lock',
    'spikes/private_rendezvous_v3/taceo_probe/src/lib.rs',
    'spikes/private_rendezvous_v3/taceo_probe/examples/composition_vector.rs',
    'spikes/private_rendezvous_v3/taceo_probe/vectors/composition_test_only_001.json',
    'spikes/private_rendezvous_v3/taceo_probe/vectors/hash_to_field_rfc9380_001.json',
    'spikes/private_rendezvous_v3/taceo_probe/vectors/v3_matrix_coverage.json',
    'spikes/private_rendezvous_v3/taceo_probe/web/rfc9380_browser_verifier.js',
    'spikes/private_rendezvous_v3/taceo_probe/web/verify_node.mjs',
    'spikes/private_rendezvous_v3/taceo_probe/web/verify_browsers.mjs'
)
if ($snapshot.files.Count -ne $expectedPaths.Count) {
    $errors.Add("expected $($expectedPaths.Count) review files, found $($snapshot.files.Count)")
}
$actualPaths = @($snapshot.files | ForEach-Object { [string]$_.path })
if (Compare-Object $expectedPaths $actualPaths) {
    $errors.Add('review snapshot paths do not match the required v2 package')
}
$seenPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($entry in $snapshot.files) {
    if (-not $seenPaths.Add([string]$entry.path)) {
        $errors.Add("duplicate review path: $($entry.path)")
        continue
    }
    $path = Join-Path $repositoryRoot ([string]$entry.path)
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $errors.Add("missing review file: $($entry.path)")
        continue
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne [string]$entry.sha256) {
        $errors.Add("hash mismatch: $($entry.path)")
    }
}

if ($ledger.schema -ne 'scotoma-v3-findings-ledger-v1' -or $ledger.decision -ne 'NO-GO') {
    $errors.Add('unexpected findings ledger schema or decision')
}
$expected = 1..12 | ForEach-Object { 'V3-R{0:d2}' -f $_ }
$actualIds = @($ledger.findings | ForEach-Object { [string]$_.id } | Sort-Object -Unique)
if (Compare-Object $expected $actualIds) {
    $errors.Add('findings ledger must contain V3-R01 through V3-R12 exactly once')
}
$allowedStatuses = @('blocked', 'partial', 'open', 'documented_risk')
foreach ($finding in $ledger.findings) {
    if ($finding.status -notin $allowedStatuses) {
        $errors.Add("invalid status for $($finding.id): $($finding.status)")
    }
    if ([String]::IsNullOrWhiteSpace([string]$finding.action) -or
        [String]::IsNullOrWhiteSpace([string]$finding.exit_condition)) {
        $errors.Add("missing action or exit condition for $($finding.id)")
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-review-snapshot-check-v1'
    files_verified = $snapshot.files.Count
    findings_verified = $ledger.findings.Count
    decision = $ledger.decision
    result = 'pass'
} | ConvertTo-Json -Compress
