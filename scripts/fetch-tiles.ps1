# One-shot download of SCIM zoom-3 base map tiles for local TerraFICS smoke
# testing. Mirrors the URL layout used at runtime by the future CDN-fetch
# milestone: https://static.satisfactory-calculator.com/imgMap/gameLayer/{build}/{z}/{x}/{y}.png
#
# Usage:
#   .\scripts\fetch-tiles.ps1                              # default destination, auto-detect build
#   .\scripts\fetch-tiles.ps1 -Destination 'D:\tiles'      # custom path
#   .\scripts\fetch-tiles.ps1 -Build 412345                # skip auto-detect
#   .\scripts\fetch-tiles.ps1 -Zoom 4                      # download a different zoom level
#
# After it finishes:
#   $env:TERRAFICS_TILE_ROOT = '<destination>'
#   cargo run --release -p scim-render --example viewer

[CmdletBinding()]
param(
    [string]$Destination = (Join-Path $env:LOCALAPPDATA 'terrafics\tiles'),
    [string]$Build,
    [int]$Zoom = 3,
    [ValidateSet('gameLayer', 'realisticLayer')]
    [string]$Layer = 'gameLayer'
)

$ErrorActionPreference = 'Stop'
$ua = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36'

if (-not $Build) {
    # Per the live SCIM.js bundle: `this.build = "Stable"` is the default the
    # tile URL pyramid uses (`/imgMap/gameLayer/{build}/{z}/{x}/{y}.png`).
    # `buildVersion` in the page's inline JS is a different field (used for
    # the save-format parser version, not the tile URL).
    $Build = 'Stable'
    Write-Host "Using build = '$Build' (SCIM default — overridable with -Build)"
}

if ($Zoom -lt 3 -or $Zoom -gt 8) {
    throw "Zoom must be in [3, 8] (SCIM's supported pyramid range). Got $Zoom."
}

$tilesPerAxis = [int][Math]::Pow(2, $Zoom)
$total = $tilesPerAxis * $tilesPerAxis
Write-Host "Downloading $total tiles at zoom $Zoom from layer '$Layer' build $Build..."
Write-Host "  destination: $Destination"

$tileDir = Join-Path $Destination $Zoom
New-Item -ItemType Directory -Force -Path $tileDir | Out-Null

$got = 0; $skipped = 0; $failed = 0
for ($x = 0; $x -lt $tilesPerAxis; $x++) {
    $xDir = Join-Path $tileDir $x
    New-Item -ItemType Directory -Force -Path $xDir | Out-Null
    for ($y = 0; $y -lt $tilesPerAxis; $y++) {
        $dst = Join-Path $xDir "$y.png"
        if (Test-Path $dst) { $skipped++; continue }
        $url = "https://static.satisfactory-calculator.com/imgMap/$Layer/$Build/$Zoom/$x/$y.png"
        try {
            Invoke-WebRequest -Uri $url -OutFile $dst -UserAgent $ua | Out-Null
            $got++
        } catch {
            Write-Warning "  failed $url : $_"
            $failed++
        }
    }
}

Write-Host ""
Write-Host "done -- downloaded $got, skipped $skipped, failed $failed"
Write-Host ""
Write-Host "To use:"
Write-Host "  `$env:TERRAFICS_TILE_ROOT = '$Destination'"
Write-Host "  cargo run --release -p scim-render --example viewer"
