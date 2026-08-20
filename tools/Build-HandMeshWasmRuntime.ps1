param(
    [string]$OutputDir = "",
    [string]$WasmBindgenVersion = "0.2.127"
)

$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$File,
        [string[]]$Arguments = @()
    )

    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Join-Path $RepoRoot "local-artifacts\handmesh-wasm"
}

$ToolRoot = Join-Path $RepoRoot "target\wasm-tools"
$WasmBindgen = Join-Path $ToolRoot "bin\wasm-bindgen.exe"
$WasmInput = Join-Path $RepoRoot "target\wasm32-unknown-unknown\release\rusty_matter_handmesh_wasm.wasm"

Push-Location $RepoRoot
try {
    if (-not (Test-Path $WasmBindgen)) {
        Invoke-Checked "install wasm-bindgen" "cargo" @(
            "install",
            "wasm-bindgen-cli",
            "--version",
            $WasmBindgenVersion,
            "--root",
            $ToolRoot
        )
    }

    Invoke-Checked "build handmesh wasm runtime" "cargo" @(
        "build",
        "-p",
        "rusty-matter-handmesh-wasm",
        "--target",
        "wasm32-unknown-unknown",
        "--release"
    )

    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    Invoke-Checked "bindgen handmesh wasm runtime" $WasmBindgen @(
        $WasmInput,
        "--target",
        "web",
        "--out-dir",
        $OutputDir,
        "--out-name",
        "rusty_matter_handmesh_wasm"
    )

    Get-ChildItem $OutputDir | Select-Object FullName, Length
} finally {
    Pop-Location
}
