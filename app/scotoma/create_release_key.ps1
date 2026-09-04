param(
    [string]$ToolsRoot = 'D:\Tools',
    [string]$KeystorePath = 'D:\Scotoma-secrets\scotoma-release.jks',
    [string]$Alias = 'scotoma'
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
Write-Output 'keytool will request the passwords and certificate identity interactively.'
Write-Output 'Use unique strong passwords and store them in an offline password manager.'
& $keytool -genkeypair -v -keystore $resolvedKey -alias $Alias `
    -keyalg RSA -keysize 4096 -validity 10000
if ($LASTEXITCODE -ne 0) { throw "keytool failed ($LASTEXITCODE)" }

Write-Output "Created release identity: $resolvedKey"
Write-Output 'Back up the keystore and passwords separately before publishing any build.'
Write-Output 'Copy android/key.properties.example to android/key.properties and fill it locally.'
