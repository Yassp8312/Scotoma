param(
    [ValidateSet('Check', 'Android', 'AndroidTest', 'AndroidRelease', 'Web', 'WebRuntime')]
    [string]$Mode = 'Check',
    [string]$ToolsRoot = 'D:\Tools',
    [string]$DeviceId,
    [string]$RelayUrl = 'ws://127.0.0.1:8787/v2/connect'
)

$ErrorActionPreference = 'Stop'
$names = @(
    'PATH', 'PUB_CACHE', 'TEMP', 'TMP', 'ANDROID_HOME', 'ANDROID_SDK_ROOT',
    'ANDROID_USER_HOME', 'GRADLE_USER_HOME'
)
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
    $bridgeRoot = Join-Path $PSScriptRoot '..\..\packages\scotoma_bridge'
    $flutter = Join-Path $ToolsRoot 'flutter\bin\flutter.bat'
    $dart = Join-Path $ToolsRoot 'flutter\bin\cache\dart-sdk\bin\dart.exe'
    $env:PUB_CACHE = Join-Path $ToolsRoot 'pub-cache'
    $env:TEMP = Join-Path $ToolsRoot 'java-temp'
    $env:TMP = $env:TEMP
    $env:ANDROID_HOME = Join-Path $ToolsRoot 'android-sdk'
    $env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
    $env:ANDROID_USER_HOME = Join-Path $ToolsRoot 'android-user'
    $env:GRADLE_USER_HOME = Join-Path $ToolsRoot 'gradle-cache'
    $env:PATH = (@((Join-Path $ToolsRoot 'flutter\bin'), $saved['PATH']) -join ';')

    if ($Mode -eq 'AndroidTest' -and [string]::IsNullOrWhiteSpace($DeviceId)) {
        throw 'Supply -DeviceId for the authorized Android device.'
    }
    if ($Mode -in @('Android', 'AndroidTest', 'AndroidRelease')) {
        & (Join-Path $bridgeRoot 'verify.ps1') -Mode Android -ToolsRoot $ToolsRoot -Release
        if ($LASTEXITCODE -ne 0) { throw 'Rust Android build failed.' }
        $jni = Join-Path $PSScriptRoot 'build\rust-jni\arm64-v8a'
        New-Item -ItemType Directory -Path $jni -Force | Out-Null
        $legacyLibrary = Join-Path $jni 'libscotoma_bridge_probe.so'
        if (Test-Path -LiteralPath $legacyLibrary) {
            Remove-Item -LiteralPath $legacyLibrary -Force
        }
        Copy-Item -LiteralPath (Join-Path $bridgeRoot 'rust\target\aarch64-linux-android\release\libscotoma_bridge.so') -Destination $jni -Force
    }
    if ($Mode -in @('AndroidRelease', 'Web', 'WebRuntime')) {
        if ($RelayUrl.Length -gt 2048 -or $RelayUrl.Contains("`r") -or
            $RelayUrl.Contains("`n") -or $RelayUrl.Contains('#') -or
            ($RelayUrl -notmatch '^wss://' -and
             $RelayUrl -notmatch '^ws://(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?/')) {
            throw 'Release/web relay must use wss://, except ws:// on an explicit loopback host.'
        }
    }
    if ($Mode -in @('Web', 'WebRuntime')) {
        & (Join-Path $bridgeRoot 'verify.ps1') -Mode Web -ToolsRoot $ToolsRoot
        if ($LASTEXITCODE -ne 0) { throw 'Rust web build failed.' }
    }
    if ($Mode -eq 'AndroidRelease' -and $RelayUrl -notmatch '^wss://') {
        throw 'Android release builds require an explicit public wss:// relay URL.'
    }

    Invoke-Checked $flutter @('pub', 'get', '--enforce-lockfile')
    switch ($Mode) {
        'Check' {
            Invoke-Checked $dart @('format', '--output=none', '--set-exit-if-changed', 'lib', 'test', 'integration_test', 'bin')
            Invoke-Checked $flutter @('analyze', '--no-pub')
            Invoke-Checked $flutter @('test', '--no-pub', '--reporter', 'expanded')
            Invoke-Checked 'node' @('test/runtime_supervisor_test.mjs')
        }
        'Android' {
            Invoke-Checked $flutter @('build', 'apk', '--debug', '--target-platform', 'android-arm64', '--no-pub')
        }
        'AndroidTest' {
            Invoke-Checked $flutter @('test', 'integration_test/session_start_test.dart', '-d', $DeviceId, '--no-pub')
        }
        'AndroidRelease' {
            if (-not (Test-Path -LiteralPath 'android\key.properties' -PathType Leaf)) {
                throw 'Create android/key.properties and the protected release keystore first.'
            }
            # `flutter pub get` generates a debug-oriented Android registrant that
            # can include dev-only plugins. Remove that generated file so the
            # release build recreates it without `integration_test`.
            $androidRegistrant = 'android\app\src\main\java\io\flutter\plugins\GeneratedPluginRegistrant.java'
            if (Test-Path -LiteralPath $androidRegistrant) {
                Remove-Item -LiteralPath $androidRegistrant -Force
            }
            Invoke-Checked $flutter @(
                'build', 'apk', '--release', '--target-platform', 'android-arm64',
                '--split-per-abi', '--no-pub',
                "--dart-define=SCOTOMA_RELAY_URL=$RelayUrl"
            )
            Invoke-Checked $flutter @(
                'build', 'appbundle', '--release', '--target-platform', 'android-arm64',
                '--no-pub', "--dart-define=SCOTOMA_RELAY_URL=$RelayUrl"
            )
            & (Join-Path $PSScriptRoot 'verify_android_release.ps1') -ToolsRoot $ToolsRoot
            if ($LASTEXITCODE -ne 0) { throw 'Android release verification failed.' }
        }
        'Web' {
            Invoke-Checked $flutter @(
                'build', 'web', '--release', '--no-web-resources-cdn', '--no-pub',
                "--dart-define=SCOTOMA_RELAY_URL=$RelayUrl"
            )
            Copy-Item -LiteralPath (Join-Path $bridgeRoot 'build\frb-web\pkg') -Destination 'build\web' -Recurse -Force
            $relayUri = [Uri]$RelayUrl
            $relayAuthority = "$($relayUri.Scheme)://$($relayUri.Host)"
            if (-not $relayUri.IsDefaultPort) {
                $relayAuthority += ":$($relayUri.Port)"
            }
            $headers = @(
                '/*',
                '  Cache-Control: no-store',
                '  Pragma: no-cache',
                '  Cross-Origin-Opener-Policy: same-origin',
                '  Cross-Origin-Embedder-Policy: require-corp',
                '  Cross-Origin-Resource-Policy: same-origin',
                '  X-Content-Type-Options: nosniff',
                '  Referrer-Policy: no-referrer',
                '  Permissions-Policy: camera=(), microphone=(), geolocation=()',
                "  Content-Security-Policy: default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; connect-src 'self' $relayAuthority; img-src 'self' data: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; worker-src 'self' blob:; manifest-src 'self'"
            )
            [IO.File]::WriteAllLines(
                (Join-Path $PSScriptRoot 'build\web\_headers'),
                $headers,
                [Text.UTF8Encoding]::new($false)
            )
        }
        'WebRuntime' {
            Invoke-Checked $flutter @('build', 'web', '--release', '--no-web-resources-cdn', '--no-pub', '-t', 'lib/web_runtime_smoke.dart')
            Copy-Item -LiteralPath (Join-Path $bridgeRoot 'build\frb-web\pkg') -Destination 'build\web' -Recurse -Force
        }
    }
} finally {
    Pop-Location
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
    }
}
