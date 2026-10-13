param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath
)

$ErrorActionPreference = 'Stop'
$manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
$errors = [Collections.Generic.List[string]]::new()

if ($manifest.schema -ne 'scotoma-v3-independent-operator-set-v1') {
    $errors.Add('unexpected operator-set schema')
}
if ([string]$manifest.transport -notin @('tcp', 'https')) {
    $errors.Add('transport must be tcp or https')
}
if ([uint64]$manifest.epoch -eq 0) {
    $errors.Add('epoch must be positive')
}
$operators = @($manifest.operators)
if ($operators.Count -lt 2) {
    $errors.Add('at least two operators are required')
}
if ([int]$manifest.threshold -lt 2 -or [int]$manifest.threshold -gt $operators.Count) {
    $errors.Add('threshold must be between two and the operator count')
}
if ($manifest.independence_attested -ne $true) {
    $errors.Add('independence_attested must be true')
}

$ids = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$keys = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$endpoints = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$admins = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$accounts = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($operator in $operators) {
    if (-not $ids.Add([string]$operator.id)) { $errors.Add("duplicate operator id $($operator.id)") }
    if ([string]$operator.public_key -notmatch '^[0-9a-fA-F]{64}$') {
        $errors.Add("operator $($operator.id) has an invalid public key")
    } elseif (-not $keys.Add([string]$operator.public_key)) {
        $errors.Add("operator $($operator.id) reuses a public key")
    }
    if ($manifest.transport -eq 'tcp' -and
        [string]$operator.endpoint -notmatch '^[A-Za-z0-9.-]+:[1-9][0-9]{0,4}$') {
        $errors.Add("operator $($operator.id) has an invalid host:port endpoint")
    } elseif ($manifest.transport -eq 'tcp') {
        $port = [int]([string]$operator.endpoint).Substring(([string]$operator.endpoint).LastIndexOf(':') + 1)
        if ($port -gt 65535) {
            $errors.Add("operator $($operator.id) has a port above 65535")
        } elseif (-not $endpoints.Add([string]$operator.endpoint)) {
            $errors.Add("operator $($operator.id) reuses an endpoint")
        }
    } elseif ($manifest.transport -eq 'https') {
        $endpoint = [string]$operator.endpoint
        if ($endpoint -notmatch '^https://[A-Za-z0-9.-]+(?::(?<port>[1-9][0-9]{0,4}))?/probe$') {
            $errors.Add("operator $($operator.id) has an invalid HTTPS probe endpoint")
        } elseif ($Matches.port -and [int]$Matches.port -gt 65535) {
            $errors.Add("operator $($operator.id) has an HTTPS port above 65535")
        } elseif (-not $endpoints.Add($endpoint)) {
            $errors.Add("operator $($operator.id) reuses an endpoint")
        }
    }
    if ([string]::IsNullOrWhiteSpace([string]$operator.administrative_domain) -or
        -not $admins.Add([string]$operator.administrative_domain)) {
        $errors.Add("operator $($operator.id) lacks a unique administrative domain")
    }
    if ([string]::IsNullOrWhiteSpace([string]$operator.provider_account) -or
        -not $accounts.Add([string]$operator.provider_account)) {
        $errors.Add("operator $($operator.id) lacks a unique provider account")
    }
    if ([string]::IsNullOrWhiteSpace([string]$operator.attestation_reference) -or
        [string]$operator.attestation_reference -eq 'TODO') {
        $errors.Add("operator $($operator.id) lacks an independence attestation")
    }
}

if ($errors.Count -ne 0) {
    $errors | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-independent-operator-set-check-v1'
    epoch = [uint64]$manifest.epoch
    transport = [string]$manifest.transport
    operators = $operators.Count
    threshold = [int]$manifest.threshold
    distinct_admin_domains = $admins.Count
    distinct_provider_accounts = $accounts.Count
    result = 'pass'
} | ConvertTo-Json -Compress
