param(
    [string]$ToolsRoot = 'D:\Tools',
    [string]$KeystorePath = 'D:\Scotoma-secrets\scotoma-release.jks',
    [string]$Alias = 'scotoma',
    [switch]$GeneratePassword,
    [string]$DistinguishedName = 'CN=SCOTOMA Release, OU=SCOTOMA, O=SCOTOMA, C=CU'
)

$ErrorActionPreference = 'Stop'
$resolvedParent = [IO.Path]::GetFullPath((Split-Path -Parent $KeystorePath))
$resolvedKey = [IO.Path]::GetFullPath($KeystorePath)
if (-not $resolvedKey.StartsWith("$resolvedParent\", [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The keystore path must remain inside its selected parent directory.'
}
if (Test-Path -LiteralPath $resolvedKey) {
    throw "Refusing to replace the existing signing identity: $resolvedKey"
}

$keytool = Join-Path $ToolsRoot 'java\bin\keytool.exe'
if (-not (Test-Path -LiteralPath $keytool)) {
    $keytool = (Get-Command keytool -ErrorAction Stop).Source
}

New-Item -ItemType Directory -Path $resolvedParent -Force | Out-Null
if ($GeneratePassword) {
    $passwordBytes = [byte[]]::new(36)
    [Security.Cryptography.RandomNumberGenerator]::Fill($passwordBytes)
    $password = [Convert]::ToBase64String($passwordBytes).
        Replace('+', '-').Replace('/', '_').TrimEnd('=')
    [Array]::Clear($passwordBytes, 0, $passwordBytes.Length)

    & $keytool -genkeypair -v -keystore $resolvedKey -alias $Alias `
        -keyalg RSA -keysize 4096 -validity 10000 `
        -storepass $password -keypass $password -dname $DistinguishedName -noprompt
    if ($LASTEXITCODE -ne 0) { throw "keytool failed ($LASTEXITCODE)" }

    $propertiesPath = Join-Path $PSScriptRoot 'android\key.properties'
    $properties = @(
        "storeFile=$($resolvedKey.Replace('\', '/'))",
        "storePassword=$password",
        "keyPassword=$password",
        "keyAlias=$Alias"
    ) -join "`n"
    [IO.File]::WriteAllText($propertiesPath, "$properties`n", [Text.UTF8Encoding]::new($false))
    $password = $null

    Write-Output "Created ignored local signing configuration: $propertiesPath"
} else {
    Write-Output 'keytool will request the passwords and certificate identity interactively.'
    Write-Output 'Use unique strong passwords and store them in an offline password manager.'
    & $keytool -genkeypair -v -keystore $resolvedKey -alias $Alias `
        -keyalg RSA -keysize 4096 -validity 10000
    if ($LASTEXITCODE -ne 0) { throw "keytool failed ($LASTEXITCODE)" }
}

Write-Output "Created release identity: $resolvedKey"
$currentIdentity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
& icacls.exe $resolvedParent '/inheritance:r' '/grant:r' `
    "$currentIdentity`:(OI)(CI)F" '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '/T' | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Failed to restrict signing-directory permissions: $resolvedParent" }
if ($GeneratePassword) {
    & icacls.exe $propertiesPath '/inheritance:r' '/grant:r' `
        "$currentIdentity`:F" '*S-1-5-18:F' '*S-1-5-32-544:F' | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Failed to restrict signing-properties permissions: $propertiesPath" }
}

Write-Output 'Back up the keystore and passwords separately before publishing any build.'
if (-not $GeneratePassword) {
    Write-Output 'Copy android/key.properties.example to android/key.properties and fill it locally.'
}
