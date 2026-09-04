param(
    [string]$ToolsRoot = 'D:\Tools',
    [string]$ApkPath = 'build\app\outputs\flutter-apk\app-arm64-v8a-release.apk',
    [string]$BundlePath = 'build\app\outputs\bundle\release\app-release.aab'
)

$ErrorActionPreference = 'Stop'
$apk = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $ApkPath))
$bundle = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $BundlePath))
foreach ($artifact in @($apk, $bundle)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        throw "Missing release artifact: $artifact"
    }
}

$analyzer = Join-Path $ToolsRoot 'android-sdk\cmdline-tools\latest\bin\apkanalyzer.bat'
$signer = Join-Path $ToolsRoot 'android-sdk\build-tools\36.0.0\apksigner.bat'
$jarsigner = Join-Path $ToolsRoot 'java\bin\jarsigner.exe'

$debuggable = (& $analyzer manifest debuggable $apk).Trim()
if ($LASTEXITCODE -ne 0 -or $debuggable -ne 'false') {
    throw "Release APK is debuggable or unreadable: $debuggable"
}
$manifest = (& $analyzer manifest print $apk) -join "`n"
foreach ($required in @('android:allowBackup="false"', 'android:usesCleartextTraffic="false"')) {
    if (-not $manifest.Contains($required)) { throw "Release manifest is missing $required" }
}
$files = (& $analyzer files list $apk) -join "`n"
if (-not $files.Contains('/lib/arm64-v8a/libscotoma_bridge.so')) {
    throw 'Release APK is missing the ARM64 Rust bridge.'
}
if ($files.Contains('libscotoma_bridge_probe.so')) {
    throw 'Release APK contains the diagnostic bridge.'
}

& $signer verify --verbose --print-certs $apk
if ($LASTEXITCODE -ne 0) { throw 'APK signature verification failed.' }
$bundleVerification = (& $jarsigner '-J-Duser.language=en' '-J-Duser.country=US' -verify $bundle 2>&1) -join "`n"
if ($LASTEXITCODE -ne 0 -or $bundleVerification -notmatch 'jar verified') {
    throw 'App Bundle signature verification failed.'
}

Get-FileHash -Algorithm SHA256 -LiteralPath $apk, $bundle |
    Select-Object Algorithm, Hash, Path
