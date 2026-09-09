# Seed fuzz/corpus/fuzz_from_bytes from repository samples (PowerShell)
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$corpusDir = Join-Path $scriptDir "corpus\fuzz_from_bytes"
$samplesDir = Join-Path $scriptDir "..\samples"

if (-not (Test-Path $corpusDir)) {
    New-Item -ItemType Directory -Path $corpusDir -Force | Out-Null
}

Write-Host "Seeding fuzz corpus from $samplesDir into $corpusDir..." -ForegroundColor Cyan

Get-ChildItem -Path $samplesDir -File | ForEach-Object {
    $dest = Join-Path $corpusDir $_.Name
    $bytes = [System.IO.File]::ReadAllBytes($_.FullName)
    $takeLen = [Math]::Min($bytes.Length, 262144)
    $slice = New-Object byte[] $takeLen
    [Array]::Copy($bytes, $slice, $takeLen)
    [System.IO.File]::WriteAllBytes($dest, $slice)
    Write-Host "  - Seeded: $($_.Name) ($([Math]::Round($takeLen / 1KB, 1)) KB)" -ForegroundColor Green
}

Write-Host "`n✅ Corpus seeded. Ready to run: cargo +nightly fuzz run fuzz_from_bytes" -ForegroundColor Green
