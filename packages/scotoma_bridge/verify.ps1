param(
    [ValidateSet('Generate', 'Native', 'Wasm', 'Web', 'Android')]
    [string]$Mode = 'Native',
    [string]$ToolsRoot = 'D:\Tools',
    [ValidateSet('aarch64-linux-android', 'armv7-linux-androideabi', 'x86_64-linux-android')]
    [string[]]$AndroidTargets = @('aarch64-linux-android'),
    [switch]$Release
)

$ErrorActionPreference = 'Stop'
$names = @(
    'PATH', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'PUB_CACHE',
    'TEMP', 'TMP', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_TARGET_DIR',
    'CC_x86_64_pc_windows_gnu', 'AR_x86_64_pc_windows_gnu',
    'CC_aarch64_linux_android', 'AR_aarch64_linux_android',
    'CC_armv7_linux_androideabi', 'AR_armv7_linux_androideabi',
    'CC_x86_64_linux_android', 'AR_x86_64_linux_android',
    'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER',
    'CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER',
    'CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER',
    'RUSTC_WRAPPER', 'WASM_PACK_CACHE', 'CARGO_BUILD_JOBS'
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
    $repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
    # Rust records source locations in panic/debug metadata even for release
    # artifacts.  Map the checkout-specific prefix to a stable virtual path so
    # Android and Wasm artifacts can be compared across independent clones.
    $sourcePathRemap = "--remap-path-prefix=$repositoryRoot=/src/scotoma"
    $dart = Join-Path $ToolsRoot 'flutter\bin\cache\dart-sdk\bin\dart.exe'
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
        (Join-Path $ToolsRoot 'wasm-pack-0.15.0'),
        (Join-Path $ToolsRoot 'wasm-bindgen-0.2.127'),
        (Join-Path $ToolsRoot 'flutter\bin'),
        (Join-Path $ToolsRoot 'rustup\toolchains\1.96.0-x86_64-pc-windows-gnu\bin'),
        $saved['PATH']
    ) -join ';')
    $env:RUSTFLAGS = $null
    $env:CARGO_ENCODED_RUSTFLAGS = $null
    $profileArgs = @()
    $profileDir = 'debug'
    if ($Release) { $profileArgs = @('--release'); $profileDir = 'release' }

    if ($Mode -in @('Native', 'Generate', 'Web')) {
        $env:CC_x86_64_pc_windows_gnu = 'x86_64-w64-mingw32-clang.exe'
        $env:AR_x86_64_pc_windows_gnu = 'llvm-ar.exe'
        # Use LLVM's linker with Rust's matching bundled GNU CRT libraries.
        # Keep the repository's Windows CSPRNG backend, not a weaker fallback.
        $env:RUSTFLAGS = "$sourcePathRemap --cfg getrandom_backend=`"windows_legacy`" -C linker=x86_64-w64-mingw32-clang.exe -C link-self-contained=yes -C dlltool=llvm-dlltool.exe"
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
            Invoke-Checked 'dart' @('run', 'bin/native_boundary_smoke.dart', "rust/target/$profileDir/scotoma_bridge.dll")
        }
        'Wasm' {
            # Target-only Web Crypto cfg is in .cargo/config.toml.
            # This is NOT FRB build-web or a browser execution test.
            $env:RUSTC_WRAPPER = Join-Path $PSScriptRoot 'host_rustc.cmd'
            Invoke-Checked 'cargo' @('build', '--manifest-path', 'rust/Cargo.toml', '--locked', '--target', 'wasm32-unknown-unknown', '-j', '4')
        }
        'Web' {
            $packVersion = & (Join-Path $ToolsRoot 'wasm-pack-0.15.0\wasm-pack.exe') --version
            if ($LASTEXITCODE -ne 0 -or $packVersion -ne 'wasm-pack 0.15.0') {
                throw 'Install the pinned wasm-pack 0.15.0 binary first.'
            }
            $bindgenVersion = & (Join-Path $ToolsRoot 'wasm-bindgen-0.2.127\wasm-bindgen.exe') --version
            if ($LASTEXITCODE -ne 0 -or $bindgenVersion -ne 'wasm-bindgen 0.2.127') {
                throw 'Install the pinned wasm-bindgen 0.2.127 binary first.'
            }
            $env:WASM_PACK_CACHE = Join-Path $ToolsRoot 'wasm-pack-cache'
            $env:CARGO_BUILD_JOBS = '4'
            $env:CARGO_TARGET_DIR = Join-Path $PSScriptRoot 'build\web-rust'
            $webFlags = @(
                $sourcePathRemap,
                '-C target-feature=+atomics,+bulk-memory,+mutable-globals',
                '-C link-args=--shared-memory', '-C link-args=--max-memory=1073741824',
                '-C link-args=--import-memory', '-C link-args=--export=__heap_base',
                '-C link-args=--export=__wasm_init_tls', '-C link-args=--export=__tls_size',
                '-C link-args=--export=__tls_align', '-C link-args=--export=__tls_base',
                '--cfg getrandom_backend="wasm_js"'
            ) -join ' '
            Invoke-Checked $dart @(
                'run', 'flutter_rust_bridge', 'build-web', '--release',
                '--rust-root', 'rust', '--output', (Join-Path $PSScriptRoot 'build\frb-web'),
                '--wasm-pack-rustup-toolchain', 'nightly-2026-09-01-x86_64-pc-windows-gnu',
                '--wasm-pack-rustflags', $webFlags,
                '--cargo-build-args=-Z',
                '--cargo-build-args=target-applies-to-host',
                '--cargo-build-args=-Z', '--cargo-build-args=host-config',
                '--cargo-build-args=--locked'
            )
            # wasm-pack's no-modules target declares a top-level lexical `let`.
            # Dart's static JS interop resolves global object properties, so expose
            # the reviewed binding explicitly without using eval/new Function.
            $bindgenScript = Join-Path $PSScriptRoot 'build\frb-web\pkg\scotoma_bridge.js'
            $bindgenBody = [IO.File]::ReadAllText($bindgenScript)
            $bindgenPrefix = 'let wasm_bindgen ='
            if (-not $bindgenBody.StartsWith($bindgenPrefix)) {
                throw 'Unexpected wasm-bindgen wrapper; review the CSP compatibility patch.'
            }
            $bindgenBody = 'globalThis.wasm_bindgen =' + $bindgenBody.Substring($bindgenPrefix.Length)
            [IO.File]::WriteAllText($bindgenScript, $bindgenBody, [Text.UTF8Encoding]::new($false))
        }
        'Android' {
            $env:RUSTC_WRAPPER = Join-Path $PSScriptRoot 'host_rustc.cmd'
            $env:RUSTFLAGS = $sourcePathRemap
            $androidTargetConfig = @{
                'aarch64-linux-android' = @{
                    Cc = 'aarch64-linux-android24-clang.cmd'
                    CcVariable = 'CC_aarch64_linux_android'
                    ArVariable = 'AR_aarch64_linux_android'
                    LinkerVariable = 'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER'
                }
                'armv7-linux-androideabi' = @{
                    Cc = 'armv7a-linux-androideabi24-clang.cmd'
                    CcVariable = 'CC_armv7_linux_androideabi'
                    ArVariable = 'AR_armv7_linux_androideabi'
                    LinkerVariable = 'CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER'
                }
                'x86_64-linux-android' = @{
                    Cc = 'x86_64-linux-android24-clang.cmd'
                    CcVariable = 'CC_x86_64_linux_android'
                    ArVariable = 'AR_x86_64_linux_android'
                    LinkerVariable = 'CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER'
                }
            }
            foreach ($target in $AndroidTargets) {
                $config = $androidTargetConfig[$target]
                $compiler = Join-Path $ndk $config.Cc
                if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
                    throw "Missing Android NDK compiler for ${target}: $compiler"
                }
                [Environment]::SetEnvironmentVariable($config.CcVariable, $compiler, 'Process')
                [Environment]::SetEnvironmentVariable($config.ArVariable, (Join-Path $ndk 'llvm-ar.exe'), 'Process')
                [Environment]::SetEnvironmentVariable($config.LinkerVariable, $compiler, 'Process')
                Invoke-Checked 'cargo' (@('build', '--manifest-path', 'rust/Cargo.toml', '--locked', '--target', $target, '-j', '4') + $profileArgs)
            }
        }
    }
} finally {
    Pop-Location
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
    }
}
