param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,
    [Parameter(Mandatory = $true)]
    [string]$ClientSeedPath,
    [ValidateRange(1, 100)]
    [int]$Rounds = 5,
    [ValidateRange(100, 30000)]
    [int]$TimeoutMs = 3000
)

$ErrorActionPreference = 'Stop'
$infraRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$spikeRoot = Split-Path -Parent $infraRoot
& (Join-Path $infraRoot 'verify_operator_set.ps1') -ManifestPath $ManifestPath | Out-Null
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($manifest.transport -ne 'tcp') {
    throw 'run_public_probe.ps1 requires a tcp operator manifest; use run_public_http_probe.ps1 for https'
}
$seedPath = (Resolve-Path -LiteralPath $ClientSeedPath).Path
$binaryName = if ($IsWindows) { 'v3_public_probe_client.exe' } else { 'v3_public_probe_client' }
$binary = Join-Path $spikeRoot "target/release/$binaryName"
cargo build --locked --release --manifest-path (Join-Path $spikeRoot 'Cargo.toml') `
    --bin v3_public_probe_client
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$previousSeed = [Environment]::GetEnvironmentVariable('SCOTOMA_V3_PROBE_CLIENT_SEED_FILE')
$hasNativeErrorPreference = Test-Path Variable:PSNativeCommandUseErrorActionPreference
if ($hasNativeErrorPreference) {
    $previousNativeErrorPreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
}
$measurements = [Collections.Generic.List[object]]::new()
$failures = 0
try {
    [Environment]::SetEnvironmentVariable('SCOTOMA_V3_PROBE_CLIENT_SEED_FILE', $seedPath)
    $sequenceBase = [uint64]([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()) * 1000
    foreach ($operator in @($manifest.operators)) {
        for ($round = 1; $round -le $Rounds; $round++) {
            $sequence = $sequenceBase + [uint64]($operator.id * 100 + $round)
            $output = & $binary `
                --operator $operator.id `
                --epoch $manifest.epoch `
                --endpoint $operator.endpoint `
                --operator-key $operator.public_key `
                --sequence $sequence `
                --timeout-ms $TimeoutMs 2>&1
            if ($LASTEXITCODE -eq 0) {
                $measurements.Add(($output | ConvertFrom-Json))
            } else {
                $failures++
                $measurements.Add([ordered]@{
                    operator = [int]$operator.id
                    epoch = [uint64]$manifest.epoch
                    sequence = $sequence
                    result = 'fail'
                })
            }
        }
    }
} finally {
    [Environment]::SetEnvironmentVariable('SCOTOMA_V3_PROBE_CLIENT_SEED_FILE', $previousSeed)
    if ($hasNativeErrorPreference) {
        $PSNativeCommandUseErrorActionPreference = $previousNativeErrorPreference
    }
}

$successfulOperators = @($measurements |
    Where-Object { $_.result -eq 'pass' } |
    Select-Object -ExpandProperty operator -Unique)
$result = if ($failures -eq 0 -and $successfulOperators.Count -ge [int]$manifest.threshold) {
    'pass'
} else {
    'fail'
}
[ordered]@{
    schema = 'scotoma-v3-public-infrastructure-measurement-v1'
    measured_at = [DateTimeOffset]::UtcNow.ToString('o')
    epoch = [uint64]$manifest.epoch
    configured_operators = @($manifest.operators).Count
    successful_operators = $successfulOperators.Count
    rounds_per_operator = $Rounds
    failures = $failures
    measurements = $measurements
    result = $result
    warning = 'Research availability evidence only; no rendezvous, TOPRF or production traffic was exercised.'
} | ConvertTo-Json -Depth 6
if ($result -ne 'pass') { exit 1 }
