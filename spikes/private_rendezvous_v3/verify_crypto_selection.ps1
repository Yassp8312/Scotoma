param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$selectionPath = Join-Path $repositoryRoot 'docs/review/V3_CRYPTO_SELECTION.json'
$ceremonyPath = Join-Path $repositoryRoot 'docs/review/V3_CEREMONY_PROBE.json'
$dkgPath = Join-Path $repositoryRoot 'docs/review/V3_DKG_SELECTION.json'
$compatibilityPath = Join-Path $repositoryRoot 'docs/review/V3_DKG_TOPRF_COMPATIBILITY.json'
$platformPath = Join-Path $repositoryRoot 'docs/review/V3_PLATFORM_PROBE.json'
$selection = Get-Content -LiteralPath $selectionPath -Raw | ConvertFrom-Json
$ceremony = Get-Content -LiteralPath $ceremonyPath -Raw | ConvertFrom-Json
$dkg = Get-Content -LiteralPath $dkgPath -Raw | ConvertFrom-Json
$compatibility = Get-Content -LiteralPath $compatibilityPath -Raw | ConvertFrom-Json
$platform = Get-Content -LiteralPath $platformPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()

if ($selection.schema -ne 'scotoma-v3-crypto-selection-v1') {
    $errors.Add('unexpected cryptographic selection schema')
}
if ($selection.status -ne 'frozen-research-candidate' -or
    $selection.production_decision -ne 'NO-GO') {
    $errors.Add('the frozen selection must remain research-only and NO-GO')
}

$expected = [ordered]@{
    construction = '3HashTDH'
    reference = 'Gu-Jarecki-Kedzior-Nazarian-Xu-2024-eprint-2024-1455'
    base_suite = 'OPRF(ristretto255, SHA-512)'
    implementation = 'stef/liboprf'
    version = '0.9.4'
    tag = 'v0.9.4'
    commit = 'ba45204806daeb5b1b249d7b35ca0035cba3d641'
    license = 'LGPL-3.0-or-later'
}
foreach ($entry in $expected.GetEnumerator()) {
    if ([string]$selection.selected.($entry.Key) -cne [string]$entry.Value) {
        $errors.Add("unexpected $($entry.Key) in frozen cryptographic selection")
    }
}
if ($selection.selected.topology.operators -ne 3 -or
    $selection.selected.topology.threshold -ne 2) {
    $errors.Add('the first validation topology must remain two-of-three')
}

if ($ceremony.schema -ne 'scotoma-v3-ceremony-probe-v1' -or
    $ceremony.production_decision -ne 'NO-GO') {
    $errors.Add('unexpected ceremony probe schema or production decision')
}
if ($ceremony.inputs.liboprf_commit -cne $expected.commit -or
    $ceremony.inputs.libsodium_version -cne '1.0.20' -or
    $ceremony.inputs.libsodium_commit -cne '93a7d0d41fe2e32409b5d00386946f491750b7de') {
    $errors.Add('ceremony probe inputs do not match the frozen commits')
}
if ($ceremony.executable_checks.Count -lt 7 -or $ceremony.not_verified.Count -lt 5) {
    $errors.Add('ceremony probe evidence or limitations were removed')
}

if ($dkg.schema -ne 'scotoma-v3-dkg-selection-v1' -or
    $dkg.status -ne 'frozen-specification-candidate' -or
    $dkg.production_decision -ne 'NO-GO') {
    $errors.Add('unexpected DKG selection schema, status or production decision')
}
$expectedDkg = [ordered]@{
    protocol = 'COCKTAIL-DKG'
    version = '0.2.1'
    ciphersuite = 'COCKTAIL(Ristretto255, SHA-512)'
}
foreach ($entry in $expectedDkg.GetEnumerator()) {
    if ([string]$dkg.selected.($entry.Key) -cne [string]$entry.Value) {
        $errors.Add("unexpected $($entry.Key) in frozen DKG selection")
    }
}
if ($dkg.selected.topology.participants -ne 3 -or
    $dkg.selected.topology.threshold -ne 2 -or
    $dkg.selected.specification.commit -cne 'e5746a0bcda86413310119abb2972e6f78c374fa' -or
    $dkg.selected.specification.sha256 -cne 'e7ded83da0b54d9bf0052368ddcd2ca9ad6e257ed7fd17abf14f7e69f44e717d' -or
    $dkg.selected.vectors.commit -cne '50a8ecf2a220f4c8bdc4f085789b8e85c26829e7' -or
    $dkg.selected.vectors.sha256 -cne '04631ddfd2a39005b7709d79aa2c3bb7edb68450589aed651aef38a3cf9c6019') {
    $errors.Add('DKG topology or pinned upstream evidence changed')
}
if ($dkg.required_properties.Count -lt 6 -or
    $dkg.blocking_gates.Count -lt 6 -or
    $dkg.rejected_shortcuts.Count -lt 4) {
    $errors.Add('DKG requirements, blocking gates or rejected shortcuts were removed')
}
if ($dkg.evaluated_implementation.repository -cne 'ZcashFoundation/frost' -or
    $dkg.evaluated_implementation.pull_request -ne 1032 -or
    $dkg.evaluated_implementation.commit -cne 'a8ab5f8ba7adcd2ea4a98ceb606ccacfb4525731' -or
    $dkg.evaluated_implementation.upstream_state.declared_specification_version -cne '0.2.0' -or
    $dkg.evaluated_implementation.ristretto_vector.sha256 -cne '04631ddfd2a39005b7709d79aa2c3bb7edb68450589aed651aef38a3cf9c6019' -or
    $dkg.evaluated_implementation.ristretto_vector.matches_frozen_0_2_1_vector -ne $true -or
    $dkg.evaluated_implementation.local_test_attempt.result -cne 'not-executed-environment-blocked' -or
    $dkg.evaluated_implementation.scotoma_ci_probe.path -cne 'spikes/private_rendezvous_v3/run_cocktail_dkg_probe.sh' -or
    $dkg.evaluated_implementation.scotoma_ci_probe.status -cne 'passed' -or
    $dkg.evaluated_implementation.scotoma_ci_probe.verified_commit -cne 'dc806d63dbd5a2f7637b6152ccaa6df254f0b7fd' -or
    $dkg.evaluated_implementation.disposition -cne 'rejected-for-integration') {
    $errors.Add('the evaluated DKG implementation evidence or disposition changed')
}
if ($compatibility.schema -cne 'scotoma-v3-dkg-toprf-compatibility-v1' -or
    $compatibility.production_decision -cne 'NO-GO' -or
    $compatibility.inputs.topology -cne '2-of-3' -or
    $compatibility.compatible.Count -lt 4 -or
    $compatibility.incompatible_or_missing.Count -lt 4 -or
    $compatibility.required_context_fields.Count -lt 7 -or
    $compatibility.executable_evidence.tests -ne 20 -or
    $compatibility.executable_evidence.zero_sharing_candidate.status -cne 'phase-one-encrypted-multiprocess-model-complete' -or
    $compatibility.executable_evidence.zero_sharing_candidate.verified.Count -lt 13 -or
    $compatibility.executable_evidence.zero_sharing_candidate.integration_paths.Count -ne 2 -or
    -not ($compatibility.executable_evidence.zero_sharing_candidate.integration_paths -contains 'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/tests/secure_zero_sharing_process.rs') -or
    -not $compatibility.executable_evidence.zero_sharing_candidate.transport.Contains('non-deployable', [StringComparison]::Ordinal) -or
    -not $compatibility.next_gate.Contains('zero-sharing', [StringComparison]::Ordinal)) {
    $errors.Add('the DKG/TOPRF compatibility boundary or zero-sharing gate changed')
}

if ($platform.schema -ne 'scotoma-v3-platform-probe-v1' -or
    $platform.production_decision -ne 'NO-GO') {
    $errors.Add('unexpected platform probe schema or production decision')
}
if ($platform.pinned_inputs.liboprf_commit -cne $expected.commit -or
    $platform.pinned_inputs.libsodium_commit -cne '93a7d0d41fe2e32409b5d00386946f491750b7de' -or
    $platform.pinned_inputs.android_ndk -cne '28.2.13676358' -or
    $platform.pinned_inputs.android_api -ne 21 -or
    $platform.pinned_inputs.emsdk_version -cne '3.1.74' -or
    $platform.pinned_inputs.emsdk_commit -cne '3d6d8ee910466516a53e665b86458faa81dae9ba') {
    $errors.Add('platform probe inputs do not match the frozen toolchain')
}
if ($platform.ci_gates.Count -lt 4 -or $platform.deferred.Count -lt 5) {
    $errors.Add('platform probe gates or limitations were removed')
}

$runnerPath = Join-Path $repositoryRoot 'spikes/private_rendezvous_v3/liboprf_probe/run_linux_probe.sh'
$runner = Get-Content -LiteralPath $runnerPath -Raw
foreach ($marker in @(
    $expected.commit,
    '93a7d0d41fe2e32409b5d00386946f491750b7de',
    'stp-dkg-corrupt',
    'toprf-update-corrupt'
)) {
    if (-not $runner.Contains([string]$marker, [StringComparison]::Ordinal)) {
        $errors.Add("liboprf runner is missing required marker '$marker'")
    }
}

$platformRunnerPath = Join-Path $repositoryRoot 'spikes/private_rendezvous_v3/liboprf_probe/run_platform_builds.sh'
$platformRunner = Get-Content -LiteralPath $platformRunnerPath -Raw
foreach ($marker in @(
    $expected.commit,
    '93a7d0d41fe2e32409b5d00386946f491750b7de',
    '28.2.13676358',
    'ANDROID_API="21"',
    '3d6d8ee910466516a53e665b86458faa81dae9ba',
    'android-armv8-a.sh',
    'android-armv7-a.sh',
    'android-x86_64.sh',
    'WebAssembly and independent Rust 3HashTDH vectors differ'
)) {
    if (-not $platformRunner.Contains([string]$marker, [StringComparison]::Ordinal)) {
        $errors.Add("platform runner is missing required marker '$marker'")
    }
}
if ($selection.blocking_gates.Count -lt 6 -or $selection.prohibitions.Count -lt 3) {
    $errors.Add('selection gates or prohibitions were removed')
}

$productionInputs = @(
    'Cargo.toml',
    'packages/scotoma_bridge/rust/Cargo.toml',
    'app/scotoma/pubspec.yaml',
    'rust/scotoma-relay/Cargo.toml'
)
$forbidden = @('liboprf', '3HashTDH', 'taceo-oprf', 'cocktail-dkg', 'frost-ristretto255')
foreach ($relativePath in $productionInputs) {
    $content = Get-Content -LiteralPath (Join-Path $repositoryRoot $relativePath) -Raw
    foreach ($marker in $forbidden) {
        if ($content.Contains($marker, [StringComparison]::OrdinalIgnoreCase)) {
            $errors.Add("production input contains frozen research marker '$marker': $relativePath")
        }
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-crypto-selection-check-v1'
    construction = $selection.selected.construction
    implementation = "$($selection.selected.implementation)@$($selection.selected.commit)"
    ceremony_probe_checks = $ceremony.executable_checks.Count
    dkg_candidate = "$($dkg.selected.protocol)@$($dkg.selected.specification.commit)"
    dkg_blocking_gates = $dkg.blocking_gates.Count
    dkg_toprf_compatibility = 'partial-zero-sharing-candidate-modelled'
    platform_probe_gates = $platform.ci_gates.Count
    production_inputs_verified = $productionInputs.Count
    production_decision = $selection.production_decision
    result = 'pass'
} | ConvertTo-Json -Compress
