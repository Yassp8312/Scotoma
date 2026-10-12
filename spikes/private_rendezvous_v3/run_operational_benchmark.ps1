param(
    [ValidateRange(10, 1000000)]
    [int]$Iterations = 20000,
    [ValidateRange(1, 10000)]
    [int]$PuzzleIterations = 100,
    [ValidateRange(10, 100000)]
    [int]$ScenarioClients = 1000,
    [ValidateRange(10, 100000)]
    [int]$StateLimit = 4096
)

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifest = Join-Path $spikeRoot 'Cargo.toml'
$env:SCOTOMA_V3_OPERATIONAL_ITERATIONS = $Iterations.ToString()
$env:SCOTOMA_V3_PUZZLE_ITERATIONS = $PuzzleIterations.ToString()
$env:SCOTOMA_V3_SCENARIO_CLIENTS = $ScenarioClients.ToString()
$env:SCOTOMA_V3_STATE_LIMIT = $StateLimit.ToString()

cargo build --release --manifest-path $manifest --example operational_benchmark
if ($LASTEXITCODE -ne 0) {
    throw "v3 operational benchmark build failed with exit code $LASTEXITCODE"
}

$binary = Join-Path $spikeRoot 'target\release\examples\operational_benchmark.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    $binary = Join-Path $spikeRoot 'target/release/examples/operational_benchmark'
}
$stdout = [IO.Path]::GetTempFileName()
$stderr = [IO.Path]::GetTempFileName()
try {
    $process = Start-Process -FilePath $binary -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    [int64]$peakWorkingSet = 0
    while (-not $process.HasExited) {
        $process.Refresh()
        $peakWorkingSet = [Math]::Max($peakWorkingSet, $process.WorkingSet64)
        Start-Sleep -Milliseconds 1
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "v3 operational benchmark failed: $(Get-Content -LiteralPath $stderr -Raw)"
    }
    $result = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
    $result | Add-Member -NotePropertyName host_peak_working_set_bytes `
        -NotePropertyValue $peakWorkingSet
    $result | Add-Member -NotePropertyName warning `
        -NotePropertyValue 'Host-local synthetic baseline; not a production limit or cross-platform result.'
    $result | ConvertTo-Json -Compress -Depth 8
}
finally {
    Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
}
