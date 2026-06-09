param(
    [Parameter(Mandatory=$true)]
    [string]$GlbPath,
    [string]$OutputRoot = "local-artifacts\hand-mesh-glb-smoke"
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
if ([System.IO.Path]::IsPathRooted($OutputRoot)) {
    $ResolvedOutput = $OutputRoot
} else {
    $ResolvedOutput = Join-Path $RepoRoot $OutputRoot
}

Push-Location $RepoRoot
try {
    Invoke-Checked "GLB mesh surface extraction" "python" @(
        "tools\extract_glb_mesh_surfaces.py",
        "--glb",
        $ResolvedGlb,
        "--output-root",
        $ResolvedOutput
    )
    Write-Output "Matter GLB smoke output: $ResolvedOutput"
} finally {
    Pop-Location
}
