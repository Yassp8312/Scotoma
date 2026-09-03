param(
    [ValidateSet('Check', 'Android', 'AndroidTest', 'Web')]
    [string]$Mode = 'Check',
    [string]$ToolsRoot = 'D:\Tools',
    [string]$DeviceId
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
    if ($Mode -in @('Android', 'AndroidTest')) {
        $probeRoot = Join-Path $PSScriptRoot '..\..\spikes\client_bridge'
        & (Join-Path $probeRoot 'verify.ps1') -Mode Android -ToolsRoot $ToolsRoot -Release
        if ($LASTEXITCODE -ne 0) { throw 'Rust Android build failed.' }
        $jni = Join-Path $PSScriptRoot 'build\rust-jni\arm64-v8a'
        New-Item -ItemType Directory -Path $jni -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $probeRoot 'rust\target\aarch64-linux-android\release\libscotoma_bridge_probe.so') -Destination $jni -Force
    }

    Invoke-Checked $flutter @('pub', 'get', '--enforce-lockfile')
    switch ($Mode) {
        'Check' {
            Invoke-Checked $dart @('format', '--output=none', '--set-exit-if-changed', 'lib', 'test', 'integration_test')
            Invoke-Checked $flutter @('analyze', '--no-pub')
            Invoke-Checked $flutter @('test', '--no-pub', '--reporter', 'expanded')
        }
        'Android' {
            Invoke-Checked $flutter @('build', 'apk', '--debug', '--target-platform', 'android-arm64', '--no-pub')
        }
        'AndroidTest' {
            Invoke-Checked $flutter @('test', 'integration_test/session_start_test.dart', '-d', $DeviceId, '--no-pub')
        }
        'Web' {
            Invoke-Checked $flutter @('build', 'web', '--release', '--no-web-resources-cdn', '--no-pub')
        }
    }
} finally {
    Pop-Location
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
    }
}
