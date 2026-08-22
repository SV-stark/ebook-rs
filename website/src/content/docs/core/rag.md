---
title: AI RAG & BM25 Relevance Scoring
description: Intelligent document chunking and Okapi BM25 ranking for LLMs.
---

`ebook-rs` provides built-in tools for Large Language Model (LLM) Retrieval-Augmented Generation (RAG).

```rust
use ebook_rs::rag::{RagChunker, RagChunkConfig};

let config = RagChunkConfig {
    max_tokens: 512,
    overlap_tokens: 64,
    include_cfi_citations: true,
};

let chunks = book.to_rag_chunks(&config);
let top_chunks = RagChunker::rank_chunks_bm25(&chunks, "quantum entanglement", 5);
```\n