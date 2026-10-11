[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$gitSafeDirectory = $repositoryRoot.Replace('\', '/')

function Fail([string]$Message) {
    throw "Secret-boundary verification failed: $Message"
}

function Read-RepositoryFile([string]$RelativePath) {
    $path = Join-Path $repositoryRoot $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Fail "missing $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw
}

$dockerIgnore = Read-RepositoryFile '.dockerignore'
$requiredDockerExclusions = @(
    '.wrangler',
    '**/.env',
    '**/.env.*',
    '**/key.properties',
    '**/*.jks',
    '**/*.keystore',
    '**/*.p12',
    '**/*.pfx',
    '**/*.pem',
    '**/*.key',
    '**/google-services.json',
    '**/*service-account*.json'
)
$dockerIgnoreLines = @(
    $dockerIgnore -split "`r?`n" |
        ForEach-Object { $_.Trim() } |
        Where-Object { $_ -and -not $_.StartsWith('#') }
)
foreach ($required in $requiredDockerExclusions) {
    if ($required -notin $dockerIgnoreLines) {
        Fail ".dockerignore does not exclude $required"
    }
}

$candidateFiles = @(
    & git -c "safe.directory=$gitSafeDirectory" -C $repositoryRoot `
        ls-files --cached --others --exclude-standard
)
if ($LASTEXITCODE -ne 0) {
    Fail 'git could not enumerate repository inputs'
}
$sensitiveName = [regex]::new(
    '(?i)(^|/)(?:\.env(?:\..+)?|key\.properties|google-services\.json|.*service-account.*\.json|.*\.(?:jks|keystore|p12|pfx|pem|key))$'
)
foreach ($candidate in $candidateFiles) {
    $normalized = $candidate.Replace('\', '/')
    if ($sensitiveName.IsMatch($normalized)) {
        Fail "repository input has a secret-bearing filename: $normalized"
    }
}

$forbiddenRustLogging = @('println!', 'eprintln!', 'dbg!', 'tracing::', 'log::')
$bridgeSourceRoot = Join-Path $repositoryRoot 'packages\scotoma_bridge\rust\src'
foreach ($source in Get-ChildItem -LiteralPath $bridgeSourceRoot -Filter '*.rs' -Recurse) {
    $body = Get-Content -LiteralPath $source.FullName -Raw
    $testBoundary = $body.IndexOf('#[cfg(test)]', [StringComparison]::Ordinal)
    if ($testBoundary -ge 0) {
        $body = $body.Substring(0, $testBoundary)
    }
    foreach ($token in $forbiddenRustLogging) {
        if ($body.Contains($token, [StringComparison]::Ordinal)) {
            Fail "$($source.FullName) contains production logging token $token"
        }
    }
}

$relaySourceRoot = Join-Path $repositoryRoot 'rust\scotoma-relay\src'
foreach ($source in Get-ChildItem -LiteralPath $relaySourceRoot -Filter '*.rs') {
    if ($source.Name -in @('main.rs', 'observability.rs')) {
        continue
    }
    $body = Get-Content -LiteralPath $source.FullName -Raw
    $testBoundary = $body.IndexOf('#[cfg(test)]', [StringComparison]::Ordinal)
    if ($testBoundary -ge 0) {
        $body = $body.Substring(0, $testBoundary)
    }
    foreach ($token in $forbiddenRustLogging) {
        if ($body.Contains($token, [StringComparison]::Ordinal)) {
            Fail "$($source.FullName) contains relay logging token $token"
        }
    }
}

$relayMain = Read-RepositoryFile 'rust/scotoma-relay/src/main.rs'
if (($relayMain.Split('println!').Count - 1) -ne 3 -or $relayMain.Contains('eprintln!')) {
    Fail 'relay startup logging changed; review it before allowing new output'
}

$observability = Read-RepositoryFile 'rust/scotoma-relay/src/observability.rs'
$expectedEventTemplate = '{{\"component\":\"scotoma-relay\",\"event\":\"{}\"}}'
if (-not $observability.Contains($expectedEventTemplate, [StringComparison]::Ordinal)) {
    Fail 'relay event JSON is no longer the reviewed fixed schema'
}
if (($observability.Split('eprintln!').Count - 1) -ne 1 -or
    -not $observability.Contains('eprintln!("{event}")', [StringComparison]::Ordinal)) {
    Fail 'relay event output changed; only the fixed OperationalEvent is allowed'
}
$eventEnum = [regex]::Match(
    $observability,
    '(?s)enum\s+OperationalEvent\s*\{(?<body>.*?)\r?\n\}'
)
if (-not $eventEnum.Success) {
    Fail 'cannot inspect OperationalEvent variants'
}
foreach ($line in $eventEnum.Groups['body'].Value -split "`r?`n") {
    $variant = $line.Trim()
    if (-not $variant) {
        continue
    }
    if ($variant -notmatch '^[A-Za-z][A-Za-z0-9]*,$') {
        Fail "OperationalEvent gained a data-bearing variant: $variant"
    }
}

Write-Output 'SECRET_BOUNDARY_OK'
