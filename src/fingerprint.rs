use crate::book::Book;
use serde::{Deserialize, Serialize};

/// Stable fingerprint representation of an eBook for content deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookFingerprint {
    /// SHA-256 hash of normalized text across all spine sections (metadata independent).
    pub content_hash: String,
    /// SHA-256 hash of core metadata (title + creator + identifier).
    pub metadata_hash: String,
    /// Total number of spine reading sections.
    pub total_sections: usize,
    /// Total character count across all text sections.
    pub total_characters: usize,
}

impl BookFingerprint {
    /// Compute similarity match score (0.0 to 1.0) against another `BookFingerprint`.
    pub fn match_score(&self, other: &BookFingerprint) -> f64 {
        if self.content_hash == other.content_hash {
            return 1.0;
        }

        if self.total_sections == other.total_sections && self.total_characters > 0 {
            let char_diff = (self.total_characters as f64 - other.total_characters as f64).abs();
            let len_similarity =
                1.0 - (char_diff / (self.total_characters.max(other.total_characters) as f64));

            if self.metadata_hash == other.metadata_hash {
                return (0.6 + 0.4 * len_similarity).clamp(0.0, 1.0);
            } else {
                return (0.8 * len_similarity).clamp(0.0, 1.0);
            }
        }

        if self.metadata_hash == other.metadata_hash {
            return 0.5;
        }

        0.0
    }

    /// Check if two book fingerprints represent the same book content.
    pub fn is_duplicate_of(&self, other: &BookFingerprint) -> bool {
        self.match_score(other) >= 0.85
    }
}

/// Book Fingerprint generator engine.
pub struct FingerprintGenerator;

impl FingerprintGenerator {
    /// Generate stable `BookFingerprint` for a `Book` instance.
    pub fn generate(book: &Book) -> BookFingerprint {
        let meta = book.metadata();

        // 1. Compute Metadata Hash
        let id_str = meta.identifier.as_deref().unwrap_or("");
        let meta_raw = format!(
            "{}:{}:{}",
            meta.title.trim().to_lowercase(),
            meta.creator().trim().to_lowercase(),
            id_str.trim().to_lowercase()
        );
        let metadata_hash = hex_sha256(meta_raw.as_bytes());

        // 2. Compute Content Hash over all section plain texts
        let mut text_buf = String::new();
        let mut total_characters = 0;

        for section in &book.sections {
            let norm_text: String = section
                .plain_text
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            total_characters += section.char_count;
            text_buf.push_str(&norm_text);
        }

        let content_hash = hex_sha256(text_buf.as_bytes());

        BookFingerprint {
            content_hash,
            metadata_hash,
            total_sections: book.sections.len(),
            total_characters,
        }
    }
}

fn hex_sha256(input: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    for chunk in input.chunks(64) {
        let mut w = [0u32; 64];
        for (j, &byte) in chunk.iter().enumerate() {
            w[j / 4] |= (byte as u32) << (24 - 8 * (j % 4));
        }
        if chunk.len() < 64 {
            w[chunk.len() / 4] |= 0x80 << (24 - 8 * (chunk.len() % 4));
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(0x428a2f98)
            .wrapping_add(w[0]);

        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    state
        .iter()
        .map(|v| format!("{:08x}", v))
        .collect::<String>()
}
