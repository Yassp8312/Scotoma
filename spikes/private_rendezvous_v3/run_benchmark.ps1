param(
    [ValidateRange(1, 1000000)]
    [int]$MappingIterations = 2000,
    [ValidateRange(1, 100000)]
    [int]$NoiseIterations = 200
)

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifest = Join-Path $spikeRoot 'taceo_probe\Cargo.toml'
$env:SCOTOMA_V3_MAPPING_ITERATIONS = $MappingIterations.ToString()
$env:SCOTOMA_V3_NOISE_ITERATIONS = $NoiseIterations.ToString()

cargo build --release --manifest-path $manifest --example v3_benchmark
if ($LASTEXITCODE -ne 0) {
    throw "v3 benchmark build failed with exit code $LASTEXITCODE"
}

$binary = Join-Path $spikeRoot 'taceo_probe\target\release\examples\v3_benchmark.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    $binary = Join-Path $spikeRoot 'taceo_probe\target\release\examples\v3_benchmark'
}
$stdout = [IO.Path]::GetTempFileName()
$stderr = [IO.Path]::GetTempFileName()
try {
    $process = Start-Process -FilePath $binary -NoNewWindow -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    [int64]$peakWorkingSet = 0
    while (-not $process.HasExited) {
        $process.Refresh()
        $peakWorkingSet = [Math]::Max($peakWorkingSet, $process.WorkingSet64)
        Start-Sleep -Milliseconds 1
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "v3 benchmark failed: $(Get-Content -LiteralPath $stderr -Raw)"
    }
    $result = Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json
    $result | Add-Member -NotePropertyName host_peak_working_set_bytes `
        -NotePropertyValue $peakWorkingSet
    $result | Add-Member -NotePropertyName warning `
        -NotePropertyValue 'Host-local research baseline; not a production limit or cross-platform result.'
    $result | ConvertTo-Json -Compress -Depth 5
}
finally {
    Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
}
