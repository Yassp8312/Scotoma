param(
    [string]$ToolsRoot = 'D:\Tools',
    [string[]]$ApkPaths = @(
        'build\app\outputs\flutter-apk\app-armeabi-v7a-release.apk',
        'build\app\outputs\flutter-apk\app-arm64-v8a-release.apk',
        'build\app\outputs\flutter-apk\app-x86_64-release.apk'
    ),
    [string]$BundlePath = 'build\app\outputs\bundle\release\app-release.aab'
)

$ErrorActionPreference = 'Stop'
$apks = @($ApkPaths | ForEach-Object {
    [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $_))
})
$bundle = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $BundlePath))
foreach ($artifact in @($apks) + @($bundle)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        throw "Missing release artifact: $artifact"
    }
}

$analyzer = Join-Path $ToolsRoot 'android-sdk\cmdline-tools\latest\bin\apkanalyzer.bat'
$signer = Join-Path $ToolsRoot 'android-sdk\build-tools\36.0.0\apksigner.bat'
$jarsigner = Join-Path $ToolsRoot 'java\bin\jarsigner.exe'

$expectedAbiByApk = @{
    'app-armeabi-v7a-release.apk' = 'armeabi-v7a'
    'app-arm64-v8a-release.apk' = 'arm64-v8a'
    'app-x86_64-release.apk' = 'x86_64'
}
foreach ($apk in $apks) {
    $debuggable = (& $analyzer manifest debuggable $apk).Trim()
    if ($LASTEXITCODE -ne 0 -or $debuggable -ne 'false') {
        throw "Release APK is debuggable or unreadable: $apk ($debuggable)"
    }
    $manifest = (& $analyzer manifest print $apk) -join "`n"
    foreach ($required in @('android:allowBackup="false"', 'android:usesCleartextTraffic="false"')) {
        if (-not $manifest.Contains($required)) { throw "Release APK $apk is missing $required" }
    }
    $files = (& $analyzer files list $apk) -join "`n"
    $expectedAbi = $expectedAbiByApk[[IO.Path]::GetFileName($apk)]
    if ([string]::IsNullOrWhiteSpace($expectedAbi) -or
        -not $files.Contains("/lib/$expectedAbi/libscotoma_bridge.so")) {
        throw "Release APK is missing the expected $expectedAbi Rust bridge: $apk"
    }
    foreach ($otherAbi in @('armeabi-v7a', 'arm64-v8a', 'x86_64')) {
        if ($otherAbi -ne $expectedAbi -and
            $files.Contains("/lib/$otherAbi/libscotoma_bridge.so")) {
            throw "Split APK $apk unexpectedly contains the $otherAbi Rust bridge."
        }
    }
    if ($files.Contains('libscotoma_bridge_probe.so')) {
        throw "Release APK contains the diagnostic bridge: $apk"
    }
    & $signer verify --verbose --print-certs $apk
    if ($LASTEXITCODE -ne 0) { throw "APK signature verification failed: $apk" }
}
$bundleVerification = (& $jarsigner '-J-Duser.language=en' '-J-Duser.country=US' -verify $bundle 2>&1) -join "`n"
if ($LASTEXITCODE -ne 0 -or $bundleVerification -notmatch 'jar verified') {
    throw 'App Bundle signature verification failed.'
}

$bundleFiles = (& $analyzer files list $bundle) -join "`n"
foreach ($abi in @('armeabi-v7a', 'arm64-v8a', 'x86_64')) {
    if (-not $bundleFiles.Contains("/lib/$abi/libscotoma_bridge.so")) {
        throw "Release App Bundle is missing the $abi Rust bridge."
    }
}
if ($bundleFiles.Contains('libscotoma_bridge_probe.so')) {
    throw 'Release App Bundle contains the diagnostic bridge.'
}

& (Join-Path $PSScriptRoot 'verify_android_security.ps1') `
    -ToolsRoot $ToolsRoot -ApkPaths $ApkPaths
if ($LASTEXITCODE -ne 0) { throw 'Android security baseline verification failed.' }

Get-FileHash -Algorithm SHA256 -LiteralPath (@($apks) + @($bundle)) |
    Select-Object Algorithm, Hash, Path
