#!/usr/bin/env bash
# scripts/fetch_gutenberg.sh
# Automated downloader for Project Gutenberg public-domain books across multiple formats.
set -euo pipefail

COUNT=25
OUT_DIR="corpus/gutenberg"
RANDOM_SELECT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --random|-r)
      RANDOM_SELECT=true
      shift
      ;;
    -*)
      echo "Unknown flag: $1"
      exit 1
      ;;
    *)
      if [ -z "${COUNT_SET:-}" ]; then
        COUNT="$1"
        COUNT_SET=true
      elif [ -z "${OUT_DIR_SET:-}" ]; then
        OUT_DIR="$1"
        OUT_DIR_SET=true
      fi
      shift
      ;;
  esac
done

mkdir -p "${OUT_DIR}"

# Curated pool of 100+ diverse Project Gutenberg books across genres, periods & encodings
GUTENBERG_IDS=(
  # Classics & 19th Century Literature
  1342 84 11 2701 1513 145 2641 345 1232 1952
  1661 74 98 4300 174 2591 1260 46 5200 3600
  2852 160 43 205 30254 996 768 64317 219 1080
  76 2500 120 28054 215 16 1184 135 1250 161
  100 244 55 16389 1934 3825 829 45 8800 2855
  # Philosophy, Ancient & Early Modern
  2554 1497 7370 20203 140 1023 5827 824 16328 1727
  6130 58585 10676 1064 2680 1795 38326 2147 2413 23
  # Sci-Fi, Gothic, Mystery & Adventure
  35 36 42 1257 514 1400 30 730 408 1938
  62 521 164 1399 2148 2097 2814 1155 120 580
  # Poetry, Plays & Non-Fiction
  10007 2000 844 15399 236 1228 1635 1524 1998 1322
  1065 1900 171 1404 158 5000 786 863 1777 15
)

# Shuffle if random selection requested
if [ "${RANDOM_SELECT}" = true ]; then
  echo "🎲 Selecting ${COUNT} random books from a pool of ${#GUTENBERG_IDS[@]} Gutenberg titles..."
  if command -v shuf >/dev/null 2>&1; then
    SHUFFLED=($(printf "%s\n" "${GUTENBERG_IDS[@]}" | shuf))
  else
    SHUFFLED=($(printf "%s\n" "${GUTENBERG_IDS[@]}" | sort -R))
  fi
  GUTENBERG_IDS=("${SHUFFLED[@]}")
else
  echo "📥 Fetching top ${COUNT} Project Gutenberg books into '${OUT_DIR}'..."
fi

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
