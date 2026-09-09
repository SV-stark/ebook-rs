#!/usr/bin/env bash
# Seed fuzz/corpus/fuzz_from_bytes from repository samples
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS_DIR="${SCRIPT_DIR}/corpus/fuzz_from_bytes"
SAMPLES_DIR="${SCRIPT_DIR}/../samples"

mkdir -p "${CORPUS_DIR}"

echo "Seeding fuzz corpus from ${SAMPLES_DIR} into ${CORPUS_DIR}..."

for file in "${SAMPLES_DIR}"/*; do
  if [ -f "$file" ]; then
    filename=$(basename "$file")
    # Take first 256KB to keep seed inputs compact for faster mutations
    head -c 262144 "$file" > "${CORPUS_DIR}/${filename}"
    echo "  - Seeded: ${filename}"
  fi
done

echo "✅ Corpus seeded. Ready to run: cargo +nightly fuzz run fuzz_from_bytes"
