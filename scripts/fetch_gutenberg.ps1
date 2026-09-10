# scripts/fetch_gutenberg.ps1
# Automated downloader for Project Gutenberg public-domain books across multiple formats.
[CmdletBinding()]
param(
    [int]$Count = 25,
    [string]$OutDir = "corpus/gutenberg",
    [switch]$Random
)

$ErrorActionPreference = "Continue"

if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

$topIds = @(
    # Classics & 19th Century Literature
    1342, 84, 11, 2701, 1513, 145, 2641, 345, 1232, 1952,
    1661, 74, 98, 4300, 174, 2591, 1260, 46, 5200, 3600,
    2852, 160, 43, 205, 30254, 996, 768, 64317, 219, 1080,
    76, 2500, 120, 28054, 215, 16, 1184, 135, 1250, 161,
    100, 244, 55, 16389, 1934, 3825, 829, 45, 8800, 2855,
    # Philosophy, Ancient & Early Modern
    2554, 1497, 7370, 20203, 140, 1023, 5827, 824, 16328, 1727,
    6130, 58585, 10676, 1064, 2680, 1795, 38326, 2147, 2413, 23,
    # Sci-Fi, Gothic, Mystery & Adventure
    35, 36, 42, 1257, 514, 1400, 30, 730, 408, 1938,
    62, 521, 164, 1399, 2148, 2097, 2814, 1155, 120, 580,
    # Poetry, Plays & Non-Fiction
    10007, 2000, 844, 15399, 236, 1228, 1635, 1524, 1998, 1322,
    1065, 1900, 171, 1404, 158, 5000, 786, 863, 1777, 15
)

if ($Random) {
    Write-Host "🎲 Selecting $Count random books from a pool of $($topIds.Count) Gutenberg titles..." -ForegroundColor Cyan
    $selected = $topIds | Sort-Object { Get-Random } | Select-Object -First $Count
} else {
    Write-Host "📥 Fetching top $Count Project Gutenberg books into '$OutDir'..." -ForegroundColor Cyan
    $selected = $topIds | Select-Object -First $Count
}

$userAgent = "ebook-rs-fuzzer/1.0 (https://github.com/SV-stark/ebook-rs)"

$selected | ForEach-Object -Parallel {
    $id = $_
    $out = $using:OutDir
    $ua = $using:userAgent
    $epubPath = Join-Path $out "gutenberg_${id}.epub"

    if ((Test-Path $epubPath) -and ((Get-Item $epubPath).Length -gt 0)) {
        Write-Host "  ⏭️  [#$id] Already cached: $epubPath" -ForegroundColor DarkGray
        return
    }

    Write-Host "  ⬇️  [#$id] Downloading EPUB..." -ForegroundColor Yellow
    $downloaded = $false
    $urls = @(
        "https://www.gutenberg.org/ebooks/$id.epub3.images",
        "https://www.gutenberg.org/ebooks/$id.epub.images"
    )

    foreach ($url in $urls) {
        try {
            Invoke-WebRequest -Uri $url -OutFile $epubPath -UserAgent $ua -TimeoutSec 30 -ErrorAction Stop
            if ((Test-Path $epubPath) -and ((Get-Item $epubPath).Length -gt 1000)) {
                $downloaded = $true
                break
            }
        } catch {
            # Try fallback URL
        }
    }

    if ($downloaded) {
        $sizeKb = [Math]::Round((Get-Item $epubPath).Length / 1KB, 1)
        Write-Host "  ✅ [#$id] Saved successfully ($sizeKb KB)" -ForegroundColor Green
    } else {
        if (Test-Path $epubPath) { Remove-Item -Force $epubPath }
        Write-Host "  ⚠️ [#$id] Failed to download" -ForegroundColor Red
    }
} -ThrottleLimit 5

$totalFiles = (Get-ChildItem -Path $OutDir -File).Count
Write-Host "`n🎉 Ingestion complete! Total files in '$OutDir': $totalFiles" -ForegroundColor Green
Write-Host "Run stress test with:" -ForegroundColor Cyan
Write-Host "cargo run --release --all-features --example corpus_stress -- $OutDir`n" -ForegroundColor Yellow
