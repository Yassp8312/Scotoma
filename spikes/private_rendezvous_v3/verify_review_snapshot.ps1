param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '../..')).Path
$snapshotPath = Join-Path $repositoryRoot 'docs/review/V3_REVIEW_SNAPSHOT.json'
$ledgerPath = Join-Path $repositoryRoot 'docs/review/V3_FINDINGS_LEDGER.json'
$snapshot = Get-Content -LiteralPath $snapshotPath -Raw | ConvertFrom-Json
$ledger = Get-Content -LiteralPath $ledgerPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()

if ($snapshot.schema -ne 'scotoma-v3-review-snapshot-v1') {
    $errors.Add('unexpected review snapshot schema')
}
if ($snapshot.files.Count -ne 18) {
    $errors.Add("expected 18 review files, found $($snapshot.files.Count)")
}
$seenPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($entry in $snapshot.files) {
    if (-not $seenPaths.Add([string]$entry.path)) {
        $errors.Add("duplicate review path: $($entry.path)")
        continue
    }
    $path = Join-Path $repositoryRoot ([string]$entry.path)
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        $errors.Add("missing review file: $($entry.path)")
        continue
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne [string]$entry.sha256) {
        $errors.Add("hash mismatch: $($entry.path)")
    }
}

if ($ledger.schema -ne 'scotoma-v3-findings-ledger-v1' -or $ledger.decision -ne 'NO-GO') {
    $errors.Add('unexpected findings ledger schema or decision')
}
$expected = 1..12 | ForEach-Object { 'V3-R{0:d2}' -f $_ }
$actualIds = @($ledger.findings | ForEach-Object { [string]$_.id } | Sort-Object -Unique)
if (Compare-Object $expected $actualIds) {
    $errors.Add('findings ledger must contain V3-R01 through V3-R12 exactly once')
}
$allowedStatuses = @('blocked', 'partial', 'open', 'documented_risk')
foreach ($finding in $ledger.findings) {
    if ($finding.status -notin $allowedStatuses) {
        $errors.Add("invalid status for $($finding.id): $($finding.status)")
    }
    if ([String]::IsNullOrWhiteSpace([string]$finding.action) -or
        [String]::IsNullOrWhiteSpace([string]$finding.exit_condition)) {
        $errors.Add("missing action or exit condition for $($finding.id)")
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-review-snapshot-check-v1'
    files_verified = $snapshot.files.Count
    findings_verified = $ledger.findings.Count
    decision = $ledger.decision
    result = 'pass'
} | ConvertTo-Json -Compress
