param(
    [string]$ToolsRoot = 'D:\Tools',
    [string[]]$ApkPaths = @(
        'build\app\outputs\flutter-apk\app-armeabi-v7a-release.apk',
        'build\app\outputs\flutter-apk\app-arm64-v8a-release.apk',
        'build\app\outputs\flutter-apk\app-x86_64-release.apk'
    )
)

$ErrorActionPreference = 'Stop'
$analyzer = Join-Path $ToolsRoot 'android-sdk\cmdline-tools\latest\bin\apkanalyzer.bat'
if (-not (Test-Path -LiteralPath $analyzer -PathType Leaf)) {
    throw "Missing Android APK analyzer: $analyzer"
}

$packageName = 'app.scotoma.scotoma'
$allowedPermissions = @(
    'android.permission.INTERNET',
    "$packageName.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"
)
$forbiddenPackagedNames = @(
    'key.properties', 'google-services.json', '.jks', '.keystore', '.p12',
    '.pfx', '.pem', '.env', 'service-account'
)

foreach ($relativePath in $ApkPaths) {
    $apk = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $relativePath))
    if (-not (Test-Path -LiteralPath $apk -PathType Leaf)) {
        throw "Missing release APK for the security baseline: $apk"
    }

    $manifest = (& $analyzer manifest print $apk) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect release manifest: $apk" }
    foreach ($required in @(
        'android:minSdkVersion="24"',
        'android:targetSdkVersion="36"',
        'android:allowBackup="false"',
        'android:fullBackupContent="false"',
        'android:dataExtractionRules=',
        'android:usesCleartextTraffic="false"',
        "android:name=`"$packageName.MainActivity`"",
        'android:taskAffinity=""'
    )) {
        if (-not $manifest.Contains($required)) {
            throw "Security baseline missing $required in $apk"
        }
    }
    foreach ($forbidden in @(
        'android:debuggable="true"',
        'android:testOnly="true"',
        'android:requestLegacyExternalStorage="true"',
        'android.intent.category.BROWSABLE'
    )) {
        if ($manifest.Contains($forbidden)) {
            throw "Security baseline rejects $forbidden in $apk"
        }
    }

    $permissions = @(& $analyzer manifest permissions $apk | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect release permissions: $apk" }
    $unexpectedPermissions = @($permissions | Where-Object { $_ -notin $allowedPermissions })
    $missingPermissions = @($allowedPermissions | Where-Object { $_ -notin $permissions })
    if ($unexpectedPermissions.Count -gt 0 -or $missingPermissions.Count -gt 0) {
        throw "Release permission set changed in $apk. Unexpected=[$($unexpectedPermissions -join ', ')] Missing=[$($missingPermissions -join ', ')]"
    }

    $packagedFiles = @(& $analyzer files list $apk | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect packaged files: $apk" }
    foreach ($packagedFile in $packagedFiles) {
        $lower = $packagedFile.ToLowerInvariant()
        foreach ($forbiddenName in $forbiddenPackagedNames) {
            if ($lower.Contains($forbiddenName)) {
                throw "Release APK contains a forbidden secret/config filename: $packagedFile"
            }
        }
    }
}

Write-Output 'ANDROID_SECURITY_BASELINE_OK'
