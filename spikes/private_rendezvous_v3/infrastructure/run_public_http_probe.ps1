param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,
    [Parameter(Mandatory = $true)]
    [string]$ClientSeedPath,
    [ValidateSet('baseline', 'single-loss', 'recovery', 'censorship', 'rotation', 'permanent-loss')]
    [string]$Scenario = 'baseline',
    [ValidateRange(1, 100)]
    [int]$Rounds = 5,
    [ValidateRange(100, 30000)]
    [int]$TimeoutMs = 3000
)

$ErrorActionPreference = 'Stop'
$infraRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$spikeRoot = Split-Path -Parent $infraRoot
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
& (Join-Path $infraRoot 'verify_operator_set.ps1') -ManifestPath $ManifestPath | Out-Null
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($manifest.transport -ne 'https') {
    throw 'run_public_http_probe.ps1 requires an https operator manifest'
}
$manifestPathResolved = (Resolve-Path -LiteralPath $ManifestPath).Path
$manifestHash = (Get-FileHash -LiteralPath $manifestPathResolved -Algorithm SHA256).Hash.ToLowerInvariant()
$sourceCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceCommit -notmatch '^[0-9a-f]{40}$') {
    throw 'could not resolve the exact source commit'
}
$seedPath = (Resolve-Path -LiteralPath $ClientSeedPath).Path
$binaryName = if ($IsWindows) { 'v3_public_probe_http_client.exe' } else { 'v3_public_probe_http_client' }
$binary = Join-Path $spikeRoot "target/release/$binaryName"
cargo build --locked --release --manifest-path (Join-Path $spikeRoot 'Cargo.toml') `
    --bin v3_public_probe_http_client
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
            $requestHex = & $binary request `
                --operator $operator.id `
                --epoch $manifest.epoch `
                --sequence $sequence 2>&1
            if ($LASTEXITCODE -ne 0) { throw 'could not create a signed HTTP probe request' }
            $timer = [Diagnostics.Stopwatch]::StartNew()
            try {
                $httpResponse = Invoke-WebRequest `
                    -Method Post `
                    -Uri $operator.endpoint `
                    -ContentType 'text/plain' `
                    -Body ([string]$requestHex) `
                    -TimeoutSec ([Math]::Ceiling($TimeoutMs / 1000))
                $timer.Stop()
                $verified = & $binary verify `
                    --operator $operator.id `
                    --epoch $manifest.epoch `
                    --sequence $sequence `
                    --operator-key $operator.public_key `
                    --response-hex ([string]$httpResponse.Content) 2>&1
                if ($LASTEXITCODE -ne 0) { throw 'signed HTTP receipt verification failed' }
                $entry = $verified | ConvertFrom-Json
                $entry | Add-Member -NotePropertyName round -NotePropertyValue $round
                $entry | Add-Member -NotePropertyName latency_micros -NotePropertyValue ([uint64]($timer.Elapsed.TotalMilliseconds * 1000))
                $measurements.Add($entry)
            } catch {
                $timer.Stop()
                $failures++
                $measurements.Add([ordered]@{
                    operator = [int]$operator.id
                    epoch = [uint64]$manifest.epoch
                    sequence = $sequence
                    round = $round
                    latency_micros = [uint64]($timer.Elapsed.TotalMilliseconds * 1000)
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
    schema = 'scotoma-v3-public-http-infrastructure-measurement-v1'
    measured_at = [DateTimeOffset]::UtcNow.ToString('o')
    source_commit = $sourceCommit
    operator_manifest_sha256 = $manifestHash
    scenario = $Scenario
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
