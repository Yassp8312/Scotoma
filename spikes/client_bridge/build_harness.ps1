param(
    [ValidateSet('Check', 'Android', 'Web', 'AndroidTest')]
    [string]$Mode = 'Check',
    [string]$ToolsRoot = 'D:\Tools',
    [string]$DeviceId,
    [switch]$Benchmark,
    [switch]$Controller,
    [switch]$WebFault,
    [switch]$Credentials
)

$ErrorActionPreference = 'Stop'
if (@($Benchmark, $Controller, $WebFault, $Credentials).Where({ $_ }).Count -gt 1) { throw 'Choose only one diagnostic entrypoint.' }
if ($WebFault -and $Mode -ne 'Web') { throw 'WebFault is a web-only destructive diagnostic.' }
$names = @('PATH', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN',
    'PUB_CACHE', 'TEMP', 'TMP', 'CARGO_TARGET_DIR', 'CARGO_ENCODED_RUSTFLAGS',
    'RUSTFLAGS', 'JAVA_HOME', 'ANDROID_HOME', 'ANDROID_SDK_ROOT',
    'ANDROID_USER_HOME', 'GRADLE_USER_HOME', 'WASM_PACK_CACHE', 'CARGO_BUILD_JOBS')
$saved = @{}
foreach ($name in $names) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Executable failed ($LASTEXITCODE)" }
}

Push-Location $PSScriptRoot
try {
    # Call dart.exe directly: dart.bat strips embedded quotes from --cfg flags.
    $dart = Join-Path $ToolsRoot 'flutter\bin\cache\dart-sdk\bin\dart.exe'
    $env:CARGO_HOME = Join-Path $ToolsRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $ToolsRoot 'rustup'
    $env:RUSTUP_TOOLCHAIN = '1.96.0-x86_64-pc-windows-gnu'
    $env:PUB_CACHE = Join-Path $ToolsRoot 'pub-cache'
    $env:TEMP = Join-Path $ToolsRoot 'java-temp'
    $env:TMP = $env:TEMP
    $env:ANDROID_HOME = Join-Path $ToolsRoot 'android-sdk'
    $env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
    $env:ANDROID_USER_HOME = Join-Path $ToolsRoot 'android-user'
    $env:GRADLE_USER_HOME = Join-Path $ToolsRoot 'gradle-cache'
    $env:WASM_PACK_CACHE = Join-Path $ToolsRoot 'wasm-pack-cache'
    $env:CARGO_BUILD_JOBS = '4'
    $env:RUSTFLAGS = $null
    $env:CARGO_ENCODED_RUSTFLAGS = $null
    $env:PATH = (@(
        (Join-Path $ToolsRoot 'rust-cargo\bin'),
        (Join-Path $ToolsRoot 'cargo\bin'),
        (Join-Path $ToolsRoot 'wasm-pack-0.15.0'),
        (Join-Path $ToolsRoot 'wasm-bindgen-0.2.127'),
        (Join-Path $ToolsRoot 'flutter\bin'),
        $saved['PATH']
    ) -join ';')

    if ($Mode -eq 'AndroidTest' -and [string]::IsNullOrWhiteSpace($DeviceId)) {
        throw 'Supply -DeviceId after authorizing installation on that Android device.'
    }
    if ($Mode -in @('Android', 'AndroidTest')) {
        & (Join-Path $PSScriptRoot 'verify.ps1') -Mode Android -ToolsRoot $ToolsRoot -Release:($Benchmark -or $Controller -or $Credentials)
        $jniDir = Join-Path $PSScriptRoot 'harness\build\rust-jni\arm64-v8a'
        New-Item -ItemType Directory -Path $jniDir -Force | Out-Null
        $rustProfile = if ($Benchmark -or $Controller -or $Credentials) { 'release' } else { 'debug' }
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot "rust\target\aarch64-linux-android\$rustProfile\libscotoma_bridge_probe.so") -Destination $jniDir
    }
    if ($Mode -eq 'Web') {
        # Fail early instead of letting wasm-pack compile native tools with
        # inherited Wasm flags or resolve a different tool version implicitly.
        $packVersion = & (Join-Path $ToolsRoot 'wasm-pack-0.15.0\wasm-pack.exe') --version
        if ($LASTEXITCODE -ne 0 -or $packVersion -ne 'wasm-pack 0.15.0') {
            throw 'Install the pinned wasm-pack 0.15.0 binary first.'
        }
        $bindgenVersion = & (Join-Path $ToolsRoot 'wasm-bindgen-0.2.127\wasm-bindgen.exe') --version
        if ($LASTEXITCODE -ne 0 -or $bindgenVersion -ne 'wasm-bindgen 0.2.127') {
            throw 'Install the pinned wasm-bindgen 0.2.127 binary first.'
        }
        $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'build\web-rust'
        Invoke-Checked $dart @('pub', 'get', '--enforce-lockfile')
        # FRB overrides RUSTFLAGS: retain EVERY threaded-memory flag and the RNG cfg.
        $webFlags = @(
            '-C target-feature=+atomics,+bulk-memory,+mutable-globals',
            '-C link-args=--shared-memory', '-C link-args=--max-memory=1073741824',
            '-C link-args=--import-memory', '-C link-args=--export=__heap_base',
            '-C link-args=--export=__wasm_init_tls', '-C link-args=--export=__tls_size',
            '-C link-args=--export=__tls_align', '-C link-args=--export=__tls_base',
            '--cfg getrandom_backend="wasm_js"'
        ) -join ' '
        $webProfileArgs = @()
        if ($Benchmark -or $Controller -or $WebFault -or $Credentials) { $webProfileArgs = @('--release') }
        Invoke-Checked $dart (@('run', 'flutter_rust_bridge', 'build-web',
            '--rust-root', 'rust', '--output', (Join-Path $PSScriptRoot 'build\frb-web'),
            '--wasm-pack-rustup-toolchain', 'nightly-2026-09-01-x86_64-pc-windows-gnu',
            '--wasm-pack-rustflags', $webFlags, '--cargo-build-args=--locked') + $webProfileArgs)
    }

    Push-Location harness
    try {
        $entrypoint = if ($Benchmark) { 'lib/benchmark_main.dart' } else { 'lib/main.dart' }
        $integrationTest = if ($Benchmark) { 'integration_test/benchmark_test.dart' } else { 'integration_test/bridge_test.dart' }
        if ($Controller) { $entrypoint = 'lib/controller_main.dart'; $integrationTest = 'integration_test/controller_test.dart' }
        if ($WebFault) { $entrypoint = 'lib/web_fault_main.dart' }
        if ($Credentials) { $entrypoint = 'lib/credentials_main.dart'; $integrationTest = 'integration_test/credentials_test.dart' }
        Invoke-Checked 'flutter' @('pub', 'get', '--enforce-lockfile')
        switch ($Mode) {
            'Check' {
                Invoke-Checked $dart @('format', '--output=none', '--set-exit-if-changed', 'lib', 'test', 'integration_test')
                Invoke-Checked 'flutter' @('analyze', '--no-pub')
                Invoke-Checked 'flutter' @('test', '--no-pub', '--reporter', 'expanded')
            }
            'Android' {
                Invoke-Checked 'flutter' @('build', 'apk', '--debug', '--target-platform', 'android-arm64', '--no-pub', '-t', $entrypoint)
            }
            'AndroidTest' {
                Invoke-Checked 'flutter' @('test', $integrationTest, '-d', $DeviceId, '--no-pub')
            }
            'Web' {
                Invoke-Checked 'flutter' @('build', 'web', '--release', '--no-web-resources-cdn', '--no-pub', '-t', $entrypoint)
                Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'build\frb-web\pkg') -Destination 'build\web' -Recurse -Force
            }
        }
    } finally { Pop-Location }
} finally {
    Pop-Location
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
    }
}
