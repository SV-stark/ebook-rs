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
from ebook_rs import PyBook

# Open book from file path or byte buffer
book = PyBook.open("sample.epub")

print(f"Title: {book.title}")
print(f"Authors: {', '.join(book.authors)}")
print(f"Sections count: {book.section_count}")

# Extract plain text from chapter 0
text = book.get_section_text(0)

# Full text search
hits = book.search("wonderland")
for hit in hits:
    print(f"Spine: {hit.spine_index} | CFI: {hit.cfi}")

# Export RAG chunks
rag_data = book.to_rag_chunks_json(max_tokens=256)
```\n