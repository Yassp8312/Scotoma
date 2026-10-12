param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$workflowPath = Join-Path $repositoryRoot '.github/workflows/ci.yml'
$policyPath = Join-Path $repositoryRoot 'deny.toml'
$documentationPath = Join-Path $repositoryRoot 'docs/SUPPLY_CHAIN.md'
$workflow = Get-Content -LiteralPath $workflowPath -Raw
$policy = Get-Content -LiteralPath $policyPath -Raw
$documentation = Get-Content -LiteralPath $documentationPath -Raw
$errors = [Collections.Generic.List[string]]::new()

$graphs = @(
    [ordered]@{ name = 'v3-model'; manifest = 'spikes/private_rendezvous_v3/Cargo.toml'; lockfile = 'spikes/private_rendezvous_v3/Cargo.lock'; sbom = 'scotoma-v3-model.cdx' },
    [ordered]@{ name = 'v3-taceo-probe'; manifest = 'spikes/private_rendezvous_v3/taceo_probe/Cargo.toml'; lockfile = 'spikes/private_rendezvous_v3/taceo_probe/Cargo.lock'; sbom = 'scotoma-v3-taceo-probe.cdx' },
    [ordered]@{ name = 'v3-fuzz'; manifest = 'spikes/private_rendezvous_v3/fuzz/Cargo.toml'; lockfile = 'spikes/private_rendezvous_v3/fuzz/Cargo.lock'; sbom = 'scotoma-v3-fuzz.cdx' },
    [ordered]@{ name = 'v3-liboprf-vector'; manifest = 'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/Cargo.toml'; lockfile = 'spikes/private_rendezvous_v3/liboprf_probe/rust_vector/Cargo.lock'; sbom = 'scotoma-v3-liboprf-vector.cdx' }
)

foreach ($graph in $graphs) {
    foreach ($relativePath in @($graph.manifest, $graph.lockfile)) {
        $absolutePath = Join-Path $repositoryRoot $relativePath
        if (-not (Test-Path -LiteralPath $absolutePath -PathType Leaf)) {
            $errors.Add("missing locked graph input: $relativePath")
        }
    }

    $auditFragment = "cargo audit --file $($graph.lockfile)"
    $denyFragment = "cargo deny --manifest-path $($graph.manifest) check"
    $sbomFragment = "--manifest-path $($graph.manifest) --format json --spec-version 1.5 --target all --override-filename $($graph.sbom)"
    foreach ($fragment in @($auditFragment, $denyFragment, $sbomFragment)) {
        if (-not $workflow.Contains($fragment)) {
            $errors.Add("CI is missing required v3 gate: $fragment")
        }
    }

    if (-not $documentation.Contains($graph.lockfile)) {
        $errors.Add("supply-chain documentation is missing $($graph.lockfile)")
    }
}

$supplyChainStart = $workflow.IndexOf('  supply-chain:')
$deploymentStart = $workflow.IndexOf('  deployment-boundary:')
if ($supplyChainStart -lt 0 -or $deploymentStart -le $supplyChainStart) {
    $errors.Add('cannot identify the supply-chain job boundary')
} else {
    $supplyChainJob = $workflow.Substring($supplyChainStart, $deploymentStart - $supplyChainStart)
    if ($supplyChainJob -match 'continue-on-error\s*:\s*true') {
        $errors.Add('supply-chain controls must not continue after an error')
    }
    foreach ($fragment in @(
        'artifacts/sbom/production',
        'artifacts/sbom/research-v3',
        'scotoma-production-cyclonedx-${{ github.sha }}',
        'scotoma-v3-research-cyclonedx-${{ github.sha }}',
        'sha256sum *.cdx.json > SHA256SUMS',
        'if-no-files-found: error'
    )) {
        if (-not $supplyChainJob.Contains($fragment)) {
            $errors.Add("supply-chain artifact gate is missing: $fragment")
        }
    }
}

if ($policy -notmatch '(?s)\[sources\].*unknown-registry\s*=\s*"deny".*unknown-git\s*=\s*"deny"') {
    $errors.Add('deny.toml must reject unknown registries and Git sources')
}
if ($policy -notmatch '(?s)\[advisories\].*ignore\s*=\s*\[\s*\]') {
    $errors.Add('deny.toml advisory exceptions must remain empty')
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-supply-chain-check-v1'
    locked_graphs_verified = $graphs.Count
    audit_gates_verified = $graphs.Count
    policy_gates_verified = $graphs.Count
    sboms_verified = $graphs.Count
    advisory_exceptions = 0
    result = 'pass'
} | ConvertTo-Json -Compress
