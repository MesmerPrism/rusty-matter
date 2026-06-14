param(
    [Parameter(Mandatory=$true)]
    [string]$GlbPath,
    [string]$Output = "crates\rusty-matter-fields\src\planarian_mesh_asset.rs",
    [string]$DataOutput = "",
    [switch]$AllowShaMismatch
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Name,
        [Parameter(Mandatory=$true)]
        [string]$File,
        [string[]]$Arguments = @()
    )

    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$ResolvedGlb = Resolve-Path $GlbPath
if ([System.IO.Path]::IsPathRooted($Output)) {
    $ResolvedOutput = $Output
} else {
    $ResolvedOutput = Join-Path $RepoRoot $Output
}

$Arguments = @(
    "tools\convert_planarian_glb_surface.py",
    "--glb",
    $ResolvedGlb,
    "--output",
    $ResolvedOutput
)
if ($DataOutput -ne "") {
    if ([System.IO.Path]::IsPathRooted($DataOutput)) {
        $ResolvedDataOutput = $DataOutput
    } else {
        $ResolvedDataOutput = Join-Path $RepoRoot $DataOutput
    }
    $Arguments += "--data-output"
    $Arguments += $ResolvedDataOutput
}
if ($AllowShaMismatch) {
    $Arguments += "--allow-sha-mismatch"
}

Push-Location $RepoRoot
try {
    Invoke-Checked "Planarian GLB surface conversion" "python" $Arguments
    Write-Output "Matter Planaria surface module: $ResolvedOutput"
    if ($DataOutput -ne "") {
        Write-Output "Matter Planaria surface data: $ResolvedDataOutput"
    }
} finally {
    Pop-Location
}
