#!/usr/bin/env bash
# scripts/fetch_gutenberg.sh
# Automated downloader for Project Gutenberg public-domain books across multiple formats.
set -euo pipefail

COUNT="${1:-25}"
OUT_DIR="${2:-corpus/gutenberg}"

mkdir -p "${OUT_DIR}"

# 50 popular Project Gutenberg classics across various genres & eras
GUTENBERG_IDS=(
  1342   # Pride and Prejudice (Jane Austen)
  84     # Frankenstein (Mary Shelley)
  11     # Alice in Wonderland (Lewis Carroll)
  2701   # Moby Dick (Herman Melville)
  1513   # Romeo and Juliet (William Shakespeare)
  145    # Middlemarch (George Eliot)
  2641   # A Room with a View (E. M. Forster)
  345    # Dracula (Bram Stoker)
  1232   # The Prince (Niccolò Machiavelli)
  1952   # The Yellow Wallpaper (Charlotte Perkins Gilman)
  1661   # The Adventures of Sherlock Holmes (Arthur Conan Doyle)
  74     # The Adventures of Tom Sawyer (Mark Twain)
  98     # A Tale of Two Cities (Charles Dickens)
  4300   # Ulysses (James Joyce)
  174    # The Picture of Dorian Gray (Oscar Wilde)
  2591   # Grimms' Fairy Tales (Brothers Grimm)
  1260   # Jane Eyre (Charlotte Brontë)
  46     # A Christmas Carol (Charles Dickens)
  5200   # Metamorphosis (Franz Kafka)
  3600   # Complete Essays of Schopenhauer
  2852   # The Hound of the Baskervilles (Arthur Conan Doyle)
  160    # The Awakening (Kate Chopin)
  43     # The Strange Case of Dr. Jekyll and Mr. Hyde
  205    # The Adventures of Huckleberry Finn (Mark Twain)
  30254  # The Romance of Lust
  996    # Don Quixote (Miguel de Cervantes)
  768    # Wuthering Heights (Emily Brontë)
  64317  # The Great Gatsby (F. Scott Fitzgerald)
  219    # Heart of Darkness (Joseph Conrad)
  1080   # A Modest Proposal (Jonathan Swift)
)

echo "📥 Fetching ${COUNT} Project Gutenberg books into '${OUT_DIR}'..."

DOWNLOADED=0
for id in "${GUTENBERG_IDS[@]}"; do
  if [ "${DOWNLOADED}" -ge "${COUNT}" ]; then
    break
  fi

  EPUB_FILE="${OUT_DIR}/gutenberg_${id}.epub"
  if [ -f "${EPUB_FILE}" ] && [ -s "${EPUB_FILE}" ]; then
    echo "  ⏭️  [#${id}] Already cached: ${EPUB_FILE}"
    DOWNLOADED=$((DOWNLOADED + 1))
    continue
  fi

  echo "  ⬇️  [#${id}] Downloading EPUB..."
  # Download in background for parallelism
  (
    curl -s -L --retry 2 --max-time 30 \
      -A "ebook-rs-fuzzer/1.0 (https://github.com/SV-stark/ebook-rs)" \
      -o "${EPUB_FILE}" \
      "https://www.gutenberg.org/ebooks/${id}.epub3.images" || \
    curl -s -L --retry 2 --max-time 30 \
      -A "ebook-rs-fuzzer/1.0 (https://github.com/SV-stark/ebook-rs)" \
      -o "${EPUB_FILE}" \
      "https://www.gutenberg.org/ebooks/${id}.epub.images" || true

    if [ -f "${EPUB_FILE}" ] && [ -s "${EPUB_FILE}" ]; then
      echo "  ✅ [#${id}] Successfully saved."
    else
      rm -f "${EPUB_FILE}"
      echo "  ⚠️ [#${id}] Failed to download."
    fi
  ) &

  DOWNLOADED=$((DOWNLOADED + 1))

  # Limit concurrent downloads to 5 to respect Gutenberg rate limits
  if [ $((DOWNLOADED % 5)) -eq 0 ]; then
    wait
  fi
done

wait
echo ""
echo "🎉 Ingestion complete! Total files in ${OUT_DIR}: $(find "${OUT_DIR}" -type f | wc -l)"
echo "Run stress test with:"
echo "cargo run --release --all-features --example corpus_stress -- ${OUT_DIR}"
