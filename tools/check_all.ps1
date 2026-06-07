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
Push-Location $RepoRoot
try {
    Invoke-Checked "cargo fmt" "cargo" @("fmt", "--all", "--check")
    Invoke-Checked "cargo test" "cargo" @("test", "--workspace")
    Invoke-Checked "fixture validate" "cargo" @("run", "-p", "rusty-matter-fixtures", "--", "validate")
    Invoke-Checked "schema export" "cargo" @("run", "-p", "rusty-matter-schema", "--", "export", "--check")
    Invoke-Checked "Matter boundary scan" "python" @("tools\check_matter_boundaries.py")
} finally {
    Pop-Location
}
