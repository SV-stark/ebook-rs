use crate::book::Book;
use crate::cfi::Cfi;
use crate::section::Section;
use roxmltree::Document;
use serde::{Deserialize, Serialize};

/// Configuration options for the AI / RAG document chunking engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagChunkConfig {
    /// Maximum estimated tokens per chunk (1 token ≈ 4 characters / ~0.75 words). Default: 512.
    pub max_tokens: usize,
    /// Overlap estimated tokens between consecutive chunks. Default: 64.
    pub overlap_tokens: usize,
    /// Preserve heading hierarchy context (e.g., "# Chapter 1 > ## Section 2") in markdown output. Default: true.
    pub preserve_headings: bool,
    /// Include EPUB CFI citation anchors for each chunk. Default: true.
    pub include_cfi: bool,
    /// Minimum character length required for a chunk to be retained. Default: 50.
    pub min_chunk_size: usize,
}

impl Default for RagChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 64,
            preserve_headings: true,
            include_cfi: true,
            min_chunk_size: 50,
        }
    }
}

/// A semantic document chunk ready for AI embeddings, Vector DBs, and RAG retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagChunk {
    /// Unique chunk identifier (e.g., `chunk-sec-0-0`).
    pub id: String,
    /// Index of the section/spine item in the book.
    pub spine_index: usize,
    /// Chapter title or section label.
    pub chapter_title: String,
    /// Heading hierarchy trail (e.g., `["Chapter 1: Introduction", "1.1 Overview"]`).
    pub heading_hierarchy: Vec<String>,
    /// EPUB CFI citation anchor pointing to the chunk's start location.
    pub cfi: String,
    /// Clean plain text content of the chunk.
    pub text: String,
    /// Contextual markdown representation including heading trail for LLM prompt ingestion.
    pub markdown: String,
    /// Estimated token count for LLM context window calculations.
    pub token_count_estimate: usize,
    /// Book title metadata.
    pub book_title: String,
    /// Creator / Author metadata.
    pub book_author: String,
}

/// AI & RAG document chunking implementation.
pub struct RagChunker;

impl RagChunker {
    /// Chunk an entire book into AI-ready semantic RAG chunks.
    pub fn chunk_book(book: &Book, config: &RagChunkConfig) -> Vec<RagChunk> {
        let meta = book.metadata();
        let book_title = meta.title.clone();
        let book_author = meta.creators.join(", ");
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            (0..book.spine().len())
                .into_par_iter()
                .filter_map(|idx| {
                    book.get_section(idx).ok().map(|sec| {
                        let chapter_title = book
                            .toc()
                            .iter()
                            .find(|t| t.href.contains(&sec.href))
                            .map(|t| t.label.clone())
                            .unwrap_or_else(|| format!("Section {}", idx + 1));

                        Self::chunk_section(
                            &sec,
                            idx,
                            &chapter_title,
                            &book_title,
                            &book_author,
                            config,
                        )
                    })
                })
                .flatten()
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut all_chunks = Vec::new();
            for idx in 0..book.spine().len() {
                if let Ok(sec) = book.get_section(idx) {
                    let chapter_title = book
                        .toc()
                        .iter()
                        .find(|t| t.href.contains(&sec.href))
                        .map(|t| t.label.clone())
                        .unwrap_or_else(|| format!("Section {}", idx + 1));

                    let section_chunks = Self::chunk_section(
                        &sec,
                        idx,
                        &chapter_title,
                        &book_title,
                        &book_author,
                        config,
                    );
                    all_chunks.extend(section_chunks);
                }
            }
            all_chunks
        }
    }

    /// Chunk a single section into AI-ready semantic RAG chunks.
    pub fn chunk_section(
        sec: &Section,
        spine_index: usize,
        chapter_title: &str,
        book_title: &str,
        book_author: &str,
        config: &RagChunkConfig,
    ) -> Vec<RagChunk> {
        let max_chars = config.max_tokens * 4;
        let overlap_chars = config.overlap_tokens * 4;
        let mut chunks = Vec::new();

        // Extract heading stack and paragraphs from raw HTML
        let (_headings, paragraphs) = Self::extract_elements(&sec.raw_html);
        if paragraphs.is_empty() {
            // Fallback to plain text if HTML parsing yields no paragraph nodes
            let text = sec.plain_text.trim();
            if text.len() >= config.min_chunk_size {
                let cfi = Cfi::from_spine_index(spine_index, None, 0).to_string();
                let markdown = if config.preserve_headings && !chapter_title.is_empty() {
                    format!("# {}\n\n{}", chapter_title, text)
                } else {
                    text.to_string()
                };
                chunks.push(RagChunk {
                    id: format!("chunk-sec-{}-0", spine_index),
                    spine_index,
                    chapter_title: chapter_title.to_string(),
                    heading_hierarchy: vec![chapter_title.to_string()],
                    cfi,
                    text: text.to_string(),
                    markdown,
                    token_count_estimate: text.len().div_ceil(4),
                    book_title: book_title.to_string(),
                    book_author: book_author.to_string(),
                });
            }
            return chunks;
        }

        let mut current_chunk_text = String::new();
        let mut current_char_offset = 0;
        let mut chunk_start_offset = 0;
        let current_headings: Vec<String> = if !chapter_title.is_empty() {
            vec![chapter_title.to_string()]
        } else {
            Vec::new()
        };

        for para in paragraphs.iter() {
            let p_char_len = para.chars().count();
            if current_chunk_text.is_empty() {
                chunk_start_offset = current_char_offset;
            }

            let cur_char_len = current_chunk_text.chars().count();
            if !current_chunk_text.is_empty()
                && cur_char_len + p_char_len > max_chars
                && cur_char_len >= config.min_chunk_size
            {
                let cfi = if config.include_cfi {
                    Cfi::from_spine_index(spine_index, None, chunk_start_offset).to_string()
                } else {
                    String::new()
                };
                let markdown = Self::build_markdown(&current_headings, &current_chunk_text, config);

                chunks.push(RagChunk {
                    id: format!("chunk-sec-{}-{}", spine_index, chunks.len()),
                    spine_index,
                    chapter_title: chapter_title.to_string(),
                    heading_hierarchy: current_headings.clone(),
                    cfi,
                    text: current_chunk_text.trim().to_string(),
                    markdown,
                    token_count_estimate: cur_char_len.div_ceil(4),
                    book_title: book_title.to_string(),
                    book_author: book_author.to_string(),
                });

                // Apply overlap
                let total_chars = current_chunk_text.chars().count();
                let keep_char_idx = total_chars.saturating_sub(overlap_chars);
                let keep_start = current_chunk_text
                    .char_indices()
                    .nth(keep_char_idx)
                    .map(|(idx, _)| idx)
                    .unwrap_or(current_chunk_text.len());
                current_chunk_text = current_chunk_text[keep_start..].to_string();
                chunk_start_offset =
                    current_char_offset.saturating_sub(current_chunk_text.chars().count());
            }

            if !current_chunk_text.is_empty() && !current_chunk_text.ends_with('\n') {
                current_chunk_text.push('\n');
            }
            current_chunk_text.push_str(para);
            current_char_offset += p_char_len + 1;
        }

        // Flush final chunk
        let final_text = current_chunk_text.trim();
        let final_chars = final_text.chars().count();
        if final_chars >= config.min_chunk_size {
            let cfi = if config.include_cfi {
                Cfi::from_spine_index(spine_index, None, chunk_start_offset).to_string()
            } else {
                String::new()
            };
            let markdown = Self::build_markdown(&current_headings, final_text, config);

            chunks.push(RagChunk {
                id: format!("chunk-sec-{}-{}", spine_index, chunks.len()),
                spine_index,
                chapter_title: chapter_title.to_string(),
                heading_hierarchy: current_headings,
                cfi,
                text: final_text.to_string(),
                markdown,
                token_count_estimate: final_chars.div_ceil(4),
                book_title: book_title.to_string(),
                book_author: book_author.to_string(),
            });
        }

        chunks
    }

    fn build_markdown(headings: &[String], body: &str, config: &RagChunkConfig) -> String {
        if !config.preserve_headings || headings.is_empty() {
            return body.to_string();
        }
        let header_trail = headings.join(" > ");
        format!("# {}\n\n{}", header_trail, body)
    }

    fn extract_elements(raw_html: &str) -> (Vec<String>, Vec<String>) {
        let mut headings = Vec::new();
        let mut paragraphs = Vec::new();

        if let Ok(doc) = Document::parse(raw_html) {
            for node in doc.descendants() {
                if node.is_element() {
                    let name = node.tag_name().name().to_lowercase();
                    if name.starts_with('h') && name.len() == 2 {
                        let text: String = node.text().unwrap_or_default().trim().to_string();
                        if !text.is_empty() {
                            headings.push(text);
                        }
                    } else if matches!(name.as_str(), "p" | "div" | "blockquote" | "li" | "section")
                    {
                        let text: String = node
                            .descendants()
                            .filter_map(|n| n.text())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !clean.is_empty() {
                            paragraphs.push(clean);
                        }
                    }
                }
            }
        }
        (headings, paragraphs)
    }

    /// Rank RAG chunks using Okapi BM25 relevance scoring algorithm for a search query.
    pub fn rank_chunks_bm25(chunks: &[RagChunk], query: &str, top_k: usize) -> Vec<ScoredRagChunk> {
        let query_terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if query_terms.is_empty() || chunks.is_empty() {
            return Vec::new();
        }

        let num_docs = chunks.len() as f32;
        let avg_doc_len = chunks
            .iter()
            .map(|c| c.text.split_whitespace().count())
            .sum::<usize>() as f32
            / num_docs.max(1.0);

        // Compute Inverse Document Frequency (IDF) for query terms
        let mut idf_map = ahash::AHashMap::new();
        for term in &query_terms {
            let doc_freq = chunks
                .iter()
                .filter(|c| c.text.to_lowercase().contains(term))
                .count() as f32;
            let idf = ((num_docs - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();
            idf_map.insert(term.clone(), idf.max(0.0));
        }

        let k1 = 1.2f32;
        let b = 0.75f32;

        let mut scored_chunks: Vec<ScoredRagChunk> = chunks
            .iter()
            .map(|chunk| {
                let doc_words: Vec<String> = chunk
                    .text
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                    .collect();

                let doc_len = doc_words.len() as f32;
                let mut score = 0.0f32;

                for term in &query_terms {
                    let tf = doc_words.iter().filter(|w| w == &term).count() as f32;
                    if tf > 0.0 {
                        let idf = idf_map.get(term).cloned().unwrap_or(0.0);
                        let num = tf * (k1 + 1.0);
                        let den = tf + k1 * (1.0 - b + b * (doc_len / avg_doc_len.max(1.0)));
                        score += idf * (num / den);
                    }
                }

                ScoredRagChunk {
                    chunk: chunk.clone(),
                    bm25_score: score,
                }
            })
            .filter(|sc| sc.bm25_score > 0.0)
            .collect();

        scored_chunks.sort_by(|a, b| {
            b.bm25_score
                .partial_cmp(&a.bm25_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if top_k > 0 && scored_chunks.len() > top_k {
            scored_chunks.truncate(top_k);
        }

        scored_chunks
    }
}

/// A RAG document chunk with Okapi BM25 relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredRagChunk {
    pub chunk: RagChunk,
    pub bm25_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_chunker_basic() {
        let html = "<html><body><h1>Introduction</h1><p>First paragraph text of the book.</p><p>Second paragraph with more content for AI embedding.</p></body></html>";
        let sec = Section {
            index: 0,
            idref: "sec1".to_string(),
            href: "sec1.html".to_string(),
            full_path: "sec1.html".to_string(),
            raw_html: html.to_string(),
            processed_html: html.to_string(),
            plain_text: "Introduction First paragraph text of the book. Second paragraph with more content for AI embedding.".to_string(),
            plain_text_lower: "introduction first paragraph text of the book. second paragraph with more content for ai embedding.".to_string(),
            char_count: 100,
            viewport_width: None,
            viewport_height: None,
        };

        let config = RagChunkConfig {
            max_tokens: 100,
            overlap_tokens: 10,
            preserve_headings: true,
            include_cfi: true,
            min_chunk_size: 10,
        };

        let chunks =
            RagChunker::chunk_section(&sec, 0, "Chapter 1", "Test Book", "Author", &config);
        assert!(!chunks.is_empty());
        assert!(chunks[0].markdown.contains("# Chapter 1"));
        assert!(chunks[0].text.contains("First paragraph"));
        assert!(chunks[0].cfi.contains("epubcfi"));
    }

    #[test]
    fn test_bm25_ranking() {
        let chunk1 = RagChunk {
            id: "c1".to_string(),
            spine_index: 0,
            chapter_title: "Ch 1".to_string(),
            heading_hierarchy: vec![],
            cfi: "epubcfi(/6/2)".to_string(),
            text: "Quantum computing uses qubits for quantum algorithms.".to_string(),
            markdown: "Quantum computing uses qubits for quantum algorithms.".to_string(),
            token_count_estimate: 10,
            book_title: "Physics".to_string(),
            book_author: "Author".to_string(),
        };
        let chunk2 = RagChunk {
            id: "c2".to_string(),
            spine_index: 1,
            chapter_title: "Ch 2".to_string(),
            heading_hierarchy: vec![],
            cfi: "epubcfi(/6/4)".to_string(),
            text: "Classical computers use binary bits.".to_string(),
            markdown: "Classical computers use binary bits.".to_string(),
            token_count_estimate: 8,
            book_title: "Physics".to_string(),
            book_author: "Author".to_string(),
        };

        let ranked = RagChunker::rank_chunks_bm25(&[chunk1, chunk2], "quantum qubits", 10);
        assert!(!ranked.is_empty());
        assert_eq!(ranked[0].chunk.id, "c1");
        assert!(ranked[0].bm25_score > 0.0);
    }
}
