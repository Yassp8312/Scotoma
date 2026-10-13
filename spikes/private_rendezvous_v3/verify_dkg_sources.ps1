param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$selectionPath = Join-Path $repositoryRoot 'docs/review/V3_DKG_SELECTION.json'
$selection = Get-Content -LiteralPath $selectionPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()
$client = [Net.Http.HttpClient]::new()

function Get-PinnedArtifact {
    param(
        [Parameter(Mandatory)] [pscustomobject] $Artifact
    )

    $url = "https://raw.githubusercontent.com/$($Artifact.repository)/$($Artifact.commit)/$($Artifact.path)"
    $bytes = $client.GetByteArrayAsync($url).GetAwaiter().GetResult()
    $hash = [Convert]::ToHexString(
        [Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
    if ($hash -cne [string]$Artifact.sha256) {
        $errors.Add("pinned artifact hash mismatch: $url")
    }
    [Text.Encoding]::UTF8.GetString($bytes)
}

try {
    $specification = Get-PinnedArtifact -Artifact $selection.selected.specification
    $vectorText = Get-PinnedArtifact -Artifact $selection.selected.vectors
    $implementationVectorArtifact = [pscustomobject]@{
        repository = $selection.evaluated_implementation.repository
        commit = $selection.evaluated_implementation.commit
        path = $selection.evaluated_implementation.ristretto_vector.path
        sha256 = $selection.evaluated_implementation.ristretto_vector.sha256
    }
    $implementationVectorText = Get-PinnedArtifact -Artifact $implementationVectorArtifact
} finally {
    $client.Dispose()
}

foreach ($marker in @(
    '**Version**: v0.2.1',
    'COCKTAIL(Ristretto255, SHA-512)',
    'Round 3: Certification',
    'Recovery vectors'
)) {
    if (-not $specification.Contains($marker, [StringComparison]::Ordinal)) {
        $errors.Add("pinned specification is missing required marker '$marker'")
    }
}

$vectors = $vectorText | ConvertFrom-Json
$implementationVectors = $implementationVectorText | ConvertFrom-Json
if ($vectors.ciphersuite -cne 'COCKTAIL(Ristretto255, SHA-512)') {
    $errors.Add('unexpected CCTV ciphersuite')
}
if ($implementationVectors.ciphersuite -cne $vectors.ciphersuite) {
    $errors.Add('evaluated implementation uses an unexpected ciphersuite')
}
if ($selection.evaluated_implementation.ristretto_vector.sha256 -cne
    $selection.selected.vectors.sha256 -or
    $selection.evaluated_implementation.ristretto_vector.matches_frozen_0_2_1_vector -ne $true) {
    $errors.Add('evaluated implementation must retain exact frozen-vector compatibility')
}
$baseline = @($vectors.vectors | Where-Object {
    $_.n -eq 3 -and $_.t -eq 2 -and [String]::IsNullOrEmpty([string]$_.extension)
})
if ($baseline.Count -ne 1) {
    $errors.Add('CCTV must contain exactly one baseline two-of-three vector')
} else {
    $vector = $baseline[0]
    if ($vector.round1.Count -ne 3 -or $vector.round2.Count -ne 3) {
        $errors.Add('baseline vector must contain three round-one and round-two participants')
    }
    if ([string]$vector.group_public_key -notmatch '^[0-9a-f]{64}$' -or
        [string]$vector.round3.transcript_hash -notmatch '^[0-9a-f]{128}$') {
        $errors.Add('baseline vector has malformed group key or transcript hash')
    }
    if ($null -eq $vector.recovery -or
        [string]$vector.recovery.recovered_secret_share -notmatch '^[0-9a-f]{64}$' -or
        [string]$vector.recovery.recovered_verification_share -notmatch '^[0-9a-f]{64}$') {
        $errors.Add('baseline vector is missing well-formed recovery evidence')
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-dkg-upstream-check-v1'
    protocol = $selection.selected.protocol
    specification_commit = $selection.selected.specification.commit
    vector_commit = $selection.selected.vectors.commit
    evaluated_implementation = "$($selection.evaluated_implementation.repository)@$($selection.evaluated_implementation.commit)"
    evaluated_vector_compatible = $true
    ciphersuite = $vectors.ciphersuite
    topology = '2-of-3'
    recovery_vector = 'present'
    production_decision = $selection.production_decision
    result = 'pass'
} | ConvertTo-Json -Compress
