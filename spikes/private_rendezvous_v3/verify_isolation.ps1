param()

$ErrorActionPreference = 'Stop'
$spikeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $spikeRoot '..\..')).Path
$forbidden = @(
    'scotoma-private-rendezvous-v3-model',
    'scotoma-taceo-client-compile-probe',
    'SCOTOMA-v3-TACEO-TWOHASHDH-BABYJUBJUB'
)
$productionInputs = @(
    'Cargo.toml',
    'Cargo.lock',
    'app/scotoma/pubspec.yaml',
    'app/scotoma/pubspec.lock',
    'packages/scotoma_bridge/pubspec.yaml',
    'packages/scotoma_bridge/pubspec.lock',
    'packages/scotoma_bridge/rust/Cargo.toml',
    'packages/scotoma_bridge/rust/Cargo.lock'
)
$violations = [Collections.Generic.List[string]]::new()

foreach ($relativePath in $productionInputs) {
    $path = Join-Path $repositoryRoot $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        continue
    }
    $content = Get-Content -LiteralPath $path -Raw
    foreach ($needle in $forbidden) {
        if ($content.Contains($needle, [StringComparison]::OrdinalIgnoreCase)) {
            $violations.Add("$relativePath contains $needle")
        }
    }
}

$metadata = cargo metadata --format-version 1 --no-deps `
    --manifest-path (Join-Path $repositoryRoot 'Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed with exit code $LASTEXITCODE"
}
foreach ($package in $metadata.packages) {
    if ($forbidden -contains $package.name) {
        $violations.Add("production Cargo graph contains $($package.name)")
    }
}

$artifactRoots = @(
    'app/scotoma/build/app/outputs',
    'app/scotoma/build/web',
    'artifacts/release'
)
$artifacts = [Collections.Generic.List[string]]::new()
foreach ($relativeRoot in $artifactRoots) {
    $artifactRoot = Join-Path $repositoryRoot $relativeRoot
    if (Test-Path -LiteralPath $artifactRoot -PathType Container) {
        Get-ChildItem -LiteralPath $artifactRoot -File -Recurse -ErrorAction Stop |
            Where-Object { $_.Extension -in @('.apk', '.aab', '.js') } |
            ForEach-Object { $artifacts.Add($_.FullName) }
    }
}

foreach ($artifact in $artifacts) {
    $bytes = [IO.File]::ReadAllBytes($artifact)
    $ascii = [Text.Encoding]::ASCII.GetString($bytes)
    foreach ($needle in $forbidden) {
        if ($ascii.Contains($needle, [StringComparison]::OrdinalIgnoreCase)) {
            $relative = [IO.Path]::GetRelativePath($repositoryRoot, $artifact)
            $violations.Add("artifact $relative contains $needle")
        }
    }
}

if ($violations.Count -ne 0) {
    $violations | ForEach-Object { Write-Error $_ }
    exit 1
}

[ordered]@{
    schema = 'scotoma-v3-isolation-check-v1'
    production_inputs_checked = $productionInputs.Count
    production_cargo_packages_checked = $metadata.packages.Count
    built_artifacts_checked = $artifacts.Count
    forbidden_markers = $forbidden.Count
    result = 'pass'
} | ConvertTo-Json -Compress
