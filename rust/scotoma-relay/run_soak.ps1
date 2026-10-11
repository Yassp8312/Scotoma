[CmdletBinding()]
param(
    [ValidateRange(1, 10000)]
    [int]$Rounds = 50,

    [ValidateRange(1, 128)]
    [int]$RoomsPerRound = 16,

    [ValidateRange(32, 4096)]
    [int]$MaxPeakWorkingSetMiB = 384,

    [string]$ToolsRoot = 'D:\Tools'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$artifactDirectory = Join-Path $repositoryRoot 'artifacts\soak'
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$stdoutPath = Join-Path $artifactDirectory "relay-soak-$timestamp.stdout.log"
$stderrPath = Join-Path $artifactDirectory "relay-soak-$timestamp.stderr.log"
$summaryPath = Join-Path $artifactDirectory 'relay-soak-latest.json'

$savedEnvironment = @{}
$environmentNames = @(
    'CARGO_HOME',
    'RUSTUP_HOME',
    'RUSTUP_TOOLCHAIN',
    'CC_x86_64_pc_windows_gnu',
    'AR_x86_64_pc_windows_gnu',
    'RUSTFLAGS',
    'SCOTOMA_SOAK_ROUNDS',
    'SCOTOMA_SOAK_ROOMS_PER_ROUND',
    'PATH'
)
foreach ($name in $environmentNames) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    $env:CARGO_HOME = Join-Path $ToolsRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $ToolsRoot 'rustup'
    $env:RUSTUP_TOOLCHAIN = '1.96.0-x86_64-pc-windows-gnu'
    $env:CC_x86_64_pc_windows_gnu = 'x86_64-w64-mingw32-clang.exe'
    $env:AR_x86_64_pc_windows_gnu = 'llvm-ar.exe'
    $env:RUSTFLAGS = '--cfg getrandom_backend="windows_legacy" -C linker=x86_64-w64-mingw32-clang.exe -C link-self-contained=yes -C dlltool=llvm-dlltool.exe'

    $toolPaths = @(
        (Join-Path $ToolsRoot 'llvm-mingw-20260826-ucrt-x86_64\bin'),
        (Join-Path $ToolsRoot 'rust-cargo\bin'),
        (Join-Path $ToolsRoot 'cargo\bin'),
        (Join-Path $ToolsRoot 'rustup\toolchains\1.96.0-x86_64-pc-windows-gnu\bin')
    )
    $env:PATH = ($toolPaths + $savedEnvironment['PATH']) -join ';'
    $env:SCOTOMA_SOAK_ROUNDS = $Rounds.ToString([Globalization.CultureInfo]::InvariantCulture)
    $env:SCOTOMA_SOAK_ROOMS_PER_ROUND = $RoomsPerRound.ToString([Globalization.CultureInfo]::InvariantCulture)

    New-Item -ItemType Directory -Path $artifactDirectory -Force | Out-Null
    Push-Location $repositoryRoot
    try {
        cargo test -p scotoma-relay --test production_load --no-run --locked
        if ($LASTEXITCODE -ne 0) {
            throw "The relay soak test binary failed to compile (exit $LASTEXITCODE)."
        }

        $testBinary = Get-ChildItem (Join-Path $repositoryRoot 'target\debug\deps') `
            -Filter 'production_load-*.exe' |
            Sort-Object LastWriteTimeUtc -Descending |
            Select-Object -First 1
        if ($null -eq $testBinary) {
            throw 'Cargo did not produce a production_load test binary.'
        }

        $arguments = @(
            'prolonged_mixed_cleanup_soak_returns_every_gauge_to_zero',
            '--exact',
            '--ignored',
            '--nocapture',
            '--test-threads=1'
        )
        $process = Start-Process `
            -FilePath $testBinary.FullName `
            -ArgumentList $arguments `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath `
            -NoNewWindow `
            -PassThru

        [long]$peakWorkingSetBytes = 0
        while (-not $process.HasExited) {
            $process.Refresh()
            if ($process.WorkingSet64 -gt $peakWorkingSetBytes) {
                $peakWorkingSetBytes = $process.WorkingSet64
            }
            Start-Sleep -Milliseconds 100
        }
        $process.WaitForExit()
        $process.Refresh()
        if ($process.PeakWorkingSet64 -gt $peakWorkingSetBytes) {
            $peakWorkingSetBytes = $process.PeakWorkingSet64
        }

        $stdout = Get-Content -LiteralPath $stdoutPath -Raw
        $stderr = Get-Content -LiteralPath $stderrPath -Raw
        if ($stdout) {
            Write-Host $stdout.TrimEnd()
        }
        if ($process.ExitCode -ne 0) {
            if ($stderr) {
                Write-Warning $stderr.TrimEnd()
            }
            throw "The relay soak failed (exit $($process.ExitCode)). See $stdoutPath and $stderrPath."
        }
        if ($stdout -notmatch 'SCOTOMA_SOAK_OK') {
            throw "The relay soak did not emit its completion marker. See $stdoutPath."
        }

        $peakWorkingSetMiB = [Math]::Round($peakWorkingSetBytes / 1MB, 2)
        $summary = [ordered]@{
            completedAtUtc = (Get-Date).ToUniversalTime().ToString('o')
            rounds = $Rounds
            roomsPerRound = $RoomsPerRound
            totalRooms = $Rounds * $RoomsPerRound
            peakWorkingSetMiB = $peakWorkingSetMiB
            maxPeakWorkingSetMiB = $MaxPeakWorkingSetMiB
            testBinary = $testBinary.FullName
            stdoutLog = $stdoutPath
            stderrLog = $stderrPath
        }
        $summary | ConvertTo-Json | Set-Content -LiteralPath $summaryPath -Encoding utf8

        if ($peakWorkingSetMiB -gt $MaxPeakWorkingSetMiB) {
            throw "Peak working set was $peakWorkingSetMiB MiB; limit is $MaxPeakWorkingSetMiB MiB."
        }

        Write-Host "SCOTOMA_SOAK_MEMORY_OK peakMiB=$peakWorkingSetMiB limitMiB=$MaxPeakWorkingSetMiB"
        Write-Host "Summary: $summaryPath"
        Write-Host "Relay event log: $stderrPath"
    }
    finally {
        Pop-Location
    }
}
finally {
    foreach ($name in $environmentNames) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
}
