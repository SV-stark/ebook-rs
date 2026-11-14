---
title: Python SDK (PyO3)
description: High-performance Python bindings for ebook-rs.
---

### Installation
```bash
pip install ebook-rs
```

### Usage Example
```python
import json

from ebook_rs import Book

# Open book from file path or byte buffer
book = Book.open("sample.epub")

print(f"Title: {book.title}")
print(f"Authors: {', '.join(book.authors)}")
print(f"Sections count: {book.section_count}")

# Extract plain text from chapter 0
text = book.get_section(0).plain_text

# Full text search
hits = json.loads(book.search("wonderland"))
for hit in hits:
    print(f"Spine: {hit['spine_index']} | CFI: {hit['cfi']}")

# Export RAG chunks
rag_data = book.to_rag_chunks_json(max_tokens=256)
```\n