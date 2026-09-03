param(
    [ValidateSet('Generate', 'Native', 'Wasm', 'Android')]
    [string]$Mode = 'Native',
    [string]$ToolsRoot = 'D:\Tools',
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$names = @(
    'PATH', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'PUB_CACHE',
    'TEMP', 'TMP', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_TARGET_DIR',
    'CC_x86_64_pc_windows_gnu', 'AR_x86_64_pc_windows_gnu',
    'CC_aarch64_linux_android', 'AR_aarch64_linux_android',
    'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER',
    'RUSTC_WRAPPER'
)
$saved = @{}
foreach ($name in $names) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Executable failed with exit code $LASTEXITCODE"
    }
}

Push-Location $PSScriptRoot
try {
    $mingw = Join-Path $ToolsRoot 'llvm-mingw-20260826-ucrt-x86_64\bin'
    $ndk = Join-Path $ToolsRoot 'android-sdk\ndk\28.2.13676358\toolchains\llvm\prebuilt\windows-x86_64\bin'
    $env:CARGO_HOME = Join-Path $ToolsRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $ToolsRoot 'rustup'
    $env:RUSTUP_TOOLCHAIN = '1.96.0-x86_64-pc-windows-gnu'
    $env:PUB_CACHE = Join-Path $ToolsRoot 'pub-cache'
    $env:TEMP = Join-Path $ToolsRoot 'java-temp'
    $env:TMP = $env:TEMP
    $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'rust\target'
    $env:PATH = (@(
        $mingw,
        (Join-Path $ToolsRoot 'rust-cargo\bin'),
        (Join-Path $ToolsRoot 'cargo\bin'),
        (Join-Path $ToolsRoot 'flutter\bin'),
        (Join-Path $ToolsRoot 'rustup\toolchains\1.96.0-x86_64-pc-windows-gnu\bin'),
        $saved['PATH']
    ) -join ';')
    $env:RUSTFLAGS = $null
    $env:CARGO_ENCODED_RUSTFLAGS = $null
    $profileArgs = @()
    $profileDir = 'debug'
    if ($Release) { $profileArgs = @('--release'); $profileDir = 'release' }

    if ($Mode -in @('Native', 'Generate')) {
        $env:CC_x86_64_pc_windows_gnu = 'x86_64-w64-mingw32-clang.exe'
        $env:AR_x86_64_pc_windows_gnu = 'llvm-ar.exe'
        # Use LLVM's linker with Rust's matching bundled GNU CRT libraries.
        # Keep the repository's Windows CSPRNG backend, not a weaker fallback.
        $env:RUSTFLAGS = '--cfg getrandom_backend="windows_legacy" -C linker=x86_64-w64-mingw32-clang.exe -C link-self-contained=yes -C dlltool=llvm-dlltool.exe'
        Invoke-Checked 'dart' @('pub', 'get', '--enforce-lockfile')
    }

    switch ($Mode) {
        'Generate' {
            Invoke-Checked (Join-Path $ToolsRoot 'frb-2.13.0\flutter_rust_bridge_codegen.exe') @(
                'generate', '--config-file', 'flutter_rust_bridge.yaml',
                '--no-auto-upgrade-dependency', '--stop-on-error'
            )
            # FRB formats with a different default import order; normalize to
            # the crate's Rust 2024 style so regeneration passes cargo fmt.
            Invoke-Checked 'cargo' @('fmt', '--manifest-path', 'rust/Cargo.toml')
        }
        'Native' {
            Invoke-Checked 'cargo' @('fmt', '--manifest-path', 'rust/Cargo.toml', '--', '--check')
            Invoke-Checked 'cargo' @('clippy', '--manifest-path', 'rust/Cargo.toml', '--locked', '--all-targets', '-j', '4', '--', '-D', 'warnings')
            Invoke-Checked 'cargo' @('test', '--manifest-path', 'rust/Cargo.toml', '--locked', '-j', '4')
            Invoke-Checked 'cargo' (@('build', '--manifest-path', 'rust/Cargo.toml', '--locked', '-j', '4') + $profileArgs)
            Invoke-Checked 'dart' @('format', '--output=none', '--set-exit-if-changed', 'lib', 'bin')
            Invoke-Checked 'dart' @('analyze', '--fatal-infos')
            Invoke-Checked 'dart' @('run', 'bin/native_smoke.dart', "rust/target/$profileDir/scotoma_bridge_probe.dll")
            Invoke-Checked 'dart' @('run', 'bin/native_failure_smoke.dart', "rust/target/$profileDir/scotoma_bridge_probe.dll")
            Invoke-Checked 'dart' @('run', 'bin/native_credential_smoke.dart', "rust/target/$profileDir/scotoma_bridge_probe.dll")
            if ($Release) {
                Invoke-Checked 'dart' @('run', 'bin/native_benchmark.dart', "rust/target/$profileDir/scotoma_bridge_probe.dll")
            }
        }
        'Wasm' {
            # Target-only Web Crypto cfg is in .cargo/config.toml.
            # This is NOT FRB build-web or a browser execution test.
            Invoke-Checked 'cargo' @('build', '--manifest-path', 'rust/Cargo.toml', '--locked', '--target', 'wasm32-unknown-unknown', '-j', '4')
        }
        'Android' {
            $env:RUSTC_WRAPPER = Join-Path $PSScriptRoot 'host_rustc.cmd'
            $env:CC_aarch64_linux_android = Join-Path $ndk 'aarch64-linux-android24-clang.cmd'
            $env:AR_aarch64_linux_android = Join-Path $ndk 'llvm-ar.exe'
            $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $env:CC_aarch64_linux_android
            Invoke-Checked 'cargo' (@('build', '--manifest-path', 'rust/Cargo.toml', '--locked', '--target', 'aarch64-linux-android', '-j', '4') + $profileArgs)
        }
    }
} finally {
    Pop-Location
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
    }
}
