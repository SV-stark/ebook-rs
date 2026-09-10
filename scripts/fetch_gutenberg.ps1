# scripts/fetch_gutenberg.ps1
# Automated downloader for Project Gutenberg public-domain books across multiple formats.
[CmdletBinding()]
param(
    [int]$Count = 25,
    [string]$OutDir = "corpus/gutenberg"
)

$ErrorActionPreference = "Continue"

if (-not (Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

$topIds = @(
    1342, # Pride and Prejudice (Jane Austen)
    84,   # Frankenstein (Mary Shelley)
    11,   # Alice in Wonderland (Lewis Carroll)
    2701, # Moby Dick (Herman Melville)
    1513, # Romeo and Juliet (William Shakespeare)
    145,  # Middlemarch (George Eliot)
    2641, # A Room with a View (E. M. Forster)
    345,  # Dracula (Bram Stoker)
    1232, # The Prince (Niccolò Machiavelli)
    1952, # The Yellow Wallpaper (Charlotte Perkins Gilman)
    1661, # The Adventures of Sherlock Holmes (Arthur Conan Doyle)
    74,   # The Adventures of Tom Sawyer (Mark Twain)
    98,   # A Tale of Two Cities (Charles Dickens)
    4300, # Ulysses (James Joyce)
    174,  # The Picture of Dorian Gray (Oscar Wilde)
    2591, # Grimms' Fairy Tales (Brothers Grimm)
    1260, # Jane Eyre (Charlotte Brontë)
    46,   # A Christmas Carol (Charles Dickens)
    5200, # Metamorphosis (Franz Kafka)
    3600, # Complete Essays of Schopenhauer
    2852, # The Hound of the Baskervilles (Arthur Conan Doyle)
    160,  # The Awakening (Kate Chopin)
    43,   # The Strange Case of Dr. Jekyll and Mr. Hyde
    205,  # The Adventures of Huckleberry Finn (Mark Twain)
    30254,# The Romance of Lust
    996,  # Don Quixote (Miguel de Cervantes)
    768,  # Wuthering Heights (Emily Brontë)
    64317,# The Great Gatsby (F. Scott Fitzgerald)
    219,  # Heart of Darkness (Joseph Conrad)
    1080  # A Modest Proposal (Jonathan Swift)
)

$selected = $topIds | Select-Object -First $Count
Write-Host "📥 Fetching $Count Project Gutenberg books into '$OutDir'..." -ForegroundColor Cyan

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
