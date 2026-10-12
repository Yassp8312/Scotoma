param()

$ErrorActionPreference = 'Stop'
$probeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$vectorPath = Join-Path $probeRoot 'vectors/3hashtdh_test_only_001.json'
$rustManifest = Join-Path $probeRoot 'rust_vector/Cargo.toml'
$vector = Get-Content -LiteralPath $vectorPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()

if ($vector.schema -ne 'scotoma-3hashtdh-vector-v1' -or
    $vector.id -ne 'test-only-001' -or
    $vector.construction -ne '3HashTDH' -or
    $vector.suite -ne 'OPRF(ristretto255, SHA-512)') {
    $errors.Add('unexpected deterministic vector identity or suite')
}
if ($vector.responses.Count -ne 3) {
    $errors.Add('the deterministic vector must contain three indexed responses')
}
foreach ($field in @('alpha', 'beta')) {
    if ([string]$vector.$field -cnotmatch '^[0-9a-f]{64}$') {
        $errors.Add("invalid $field encoding")
    }
}
if ([string]$vector.final -cnotmatch '^[0-9a-f]{128}$') {
    $errors.Add('invalid final output encoding')
}
for ($i = 0; $i -lt $vector.responses.Count; $i++) {
    $expectedPrefix = '{0:x2}' -f ($i + 1)
    if ([string]$vector.responses[$i] -cnotmatch '^[0-9a-f]{66}$' -or
        -not ([string]$vector.responses[$i]).StartsWith($expectedPrefix, [StringComparison]::Ordinal)) {
        $errors.Add("invalid indexed response $($i + 1)")
    }
}

$rustLine = cargo run --quiet --manifest-path $rustManifest
if ($LASTEXITCODE -ne 0) {
    $errors.Add('independent Rust vector execution failed')
} else {
    $rust = $rustLine | ConvertFrom-Json
    foreach ($field in @('schema', 'id', 'alpha', 'beta', 'final')) {
        if ([string]$rust.$field -cne [string]$vector.$field) {
            $errors.Add("Rust vector mismatch: $field")
        }
    }
    if (Compare-Object @($rust.responses) @($vector.responses) -SyncWindow 0) {
        $errors.Add('Rust vector response mismatch')
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-3hashtdh-independent-vector-check-v1'
    vector = $vector.id
    responses = $vector.responses.Count
    result = 'pass'
} | ConvertTo-Json -Compress
