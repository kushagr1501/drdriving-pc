param([string]$Destination = (Join-Path $env:APPDATA 'drdriving\custom-cars'))
$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot 'web\car-pack'
$slots = @(20,21,22,23,25,26,27)
foreach ($slot in $slots) {
    if (-not (Test-Path -LiteralPath (Join-Path $source "car$slot.carhq") -PathType Leaf)) {
        throw "Missing car$slot.carhq. Rebuild the local pack using tools/car-quality/README.md."
    }
}
New-Item -ItemType Directory -Path $Destination -Force | Out-Null
$backup = Join-Path $Destination ('backup-before-detail-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
foreach ($slot in $slots) {
    $target = Join-Path $Destination "car$slot.carhq"
    if (Test-Path -LiteralPath $target) {
        New-Item -ItemType Directory -Path $backup -Force | Out-Null
        Copy-Item -LiteralPath $target -Destination $backup
    }
    $original = Join-Path $source "car$slot.carhq"
    Copy-Item -LiteralPath $original -Destination $target -Force
    if ((Get-FileHash -LiteralPath $original).Hash -ne (Get-FileHash -LiteralPath $target).Hash) {
        throw "Installed car$slot failed verification."
    }
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'tools\car-quality\CREDITS.md') -Destination (Join-Path $Destination 'DETAILED-CAR-CREDITS.md') -Force
Write-Output "Installed and verified seven detailed cars in $Destination"
if (Test-Path -LiteralPath $backup) { Write-Output "Previous detailed packages backed up to $backup" }
