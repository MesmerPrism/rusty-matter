param(
    [Parameter(Mandatory=$true)]
    [string]$GlbPath,
    [Parameter(Mandatory=$true)]
    [string]$Output,
    [int]$MeshIndex = 0,
    [int]$PrimitiveIndex = 0,
    [int]$AnimationIndex = 0,
    [int]$FrameCount = 120
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

Push-Location $RepoRoot
try {
    Invoke-Checked "GLB animated mesh surface extraction" "python" @(
        "tools\extract_glb_mesh_surface_sequence.py",
        "--glb",
        $ResolvedGlb,
        "--output",
        $ResolvedOutput,
        "--mesh-index",
        "$MeshIndex",
        "--primitive-index",
        "$PrimitiveIndex",
        "--animation-index",
        "$AnimationIndex",
        "--frame-count",
        "$FrameCount"
    )
    Write-Output "Matter animated GLB sequence output: $ResolvedOutput"
} finally {
    Pop-Location
}
