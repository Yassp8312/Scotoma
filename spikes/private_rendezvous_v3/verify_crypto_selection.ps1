param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$selectionPath = Join-Path $repositoryRoot 'docs/review/V3_CRYPTO_SELECTION.json'
$ceremonyPath = Join-Path $repositoryRoot 'docs/review/V3_CEREMONY_PROBE.json'
$platformPath = Join-Path $repositoryRoot 'docs/review/V3_PLATFORM_PROBE.json'
$selection = Get-Content -LiteralPath $selectionPath -Raw | ConvertFrom-Json
$ceremony = Get-Content -LiteralPath $ceremonyPath -Raw | ConvertFrom-Json
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
$forbidden = @('liboprf', '3HashTDH', 'taceo-oprf')
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
    platform_probe_gates = $platform.ci_gates.Count
    production_inputs_verified = $productionInputs.Count
    production_decision = $selection.production_decision
    result = 'pass'
} | ConvertTo-Json -Compress
