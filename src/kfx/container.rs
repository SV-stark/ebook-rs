use sha1_smol::Sha1;

pub const KFX_MAGIC_CONT: &[u8; 4] = b"CONT";
pub const KFX_HEADER_LEN: usize = 18;

/// A single KFX container index table entry (24 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KfxIndexEntry {
    pub entity_id: u32,
    pub type_id: u32,
    pub offset: u64,
    pub length: u64,
}

/// Parsed KFX container structure.
#[derive(Debug, Clone)]
pub struct KfxContainer {
    pub version: u16,
    pub is_ion_container: bool,
    pub doc_symbols: Vec<String>,
    pub index_entries: Vec<KfxIndexEntry>,
    pub payload_start: usize,
    pub payload: Vec<u8>,
}

impl KfxContainer {
    /// Detect if bytes start with valid KFX magic.
    pub fn is_kfx(bytes: &[u8]) -> bool {
        bytes.len() >= 4 && &bytes[0..4] == KFX_MAGIC_CONT
    }

    /// Retrieve raw byte slice for a specific index entry from the container payload.
    ///
    /// `offset`/`length` are raw little-endian values from the (attacker-controlled) KFX index
    /// table. The `u64 -> usize` conversion is checked rather than `as`-cast: on 32-bit targets
    /// (including `wasm32`, a first-class target here) a plain cast silently truncates, so a
    /// crafted `0x1_0000_0100` becomes `0x100` and yields a wrong-but-in-bounds slice instead of
    /// an error.
    pub fn get_entry_payload<'a>(&'a self, entry: &KfxIndexEntry) -> Option<&'a [u8]> {
        let len = usize::try_from(entry.length).ok()?;
        let off = usize::try_from(entry.offset).ok()?;
        if off.checked_add(len)? <= self.payload.len() {
            Some(&self.payload[off..off + len])
        } else if off >= self.payload_start {
            let rel_off = off - self.payload_start;
            if rel_off.checked_add(len)? <= self.payload.len() {
                Some(&self.payload[rel_off..rel_off + len])
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Parse raw container bytes into KfxContainer.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if !Self::is_kfx(bytes) {
            return Err("Invalid KFX container magic bytes".to_string());
        }
        if bytes.len() < KFX_HEADER_LEN {
            return Err("Truncated KFX container header".to_string());
        }

        let lossy = String::from_utf8_lossy(bytes);
        if lossy.contains("$DRM")
            || lossy.contains("DRM_V1")
            || lossy.contains("DRM_V2")
            || lossy.contains("kfx_drm")
        {
            return Err("Book is protected by Amazon KFX DRM encryption. Please remove DRM to read or convert this book.".to_string());
        }

        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let header_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let field_a = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let field_b = u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]) as usize;

        let mut is_ion_container = false;
        let mut doc_symbols = Vec::new();
        let mut curr_offset = if field_a > 0 { field_a } else { KFX_HEADER_LEN };
        let mut index_count = field_b;

        if field_b >= 4
            && field_a.saturating_add(field_b) <= bytes.len()
            && bytes[field_a..field_a + 4] == crate::kfx::ion::ION_BVM
            && let Ok(values) = crate::kfx::ion::decode_stream(&bytes[field_a..field_a + field_b])
            && let Some(info) = values.first()
        {
            is_ion_container = true;
            if let Some(idx_off) = info
                .get_field(crate::kfx::symbols::SYM_INDEX_TABLE_OFFSET as u64)
                .and_then(|v| v.as_u64())
                && let Some(idx_len) = info
                    .get_field(crate::kfx::symbols::SYM_INDEX_TABLE_LENGTH as u64)
                    .and_then(|v| v.as_u64())
            {
                curr_offset = idx_off as usize;
                index_count = (idx_len as usize) / 24;
            } else {
                index_count = 0;
            }
            if let Some(sym_off) = info
                .get_field(crate::kfx::symbols::SYM_DOC_SYMBOL_OFFSET as u64)
                .and_then(|v| v.as_u64())
                && let Some(sym_len) = info
                    .get_field(crate::kfx::symbols::SYM_DOC_SYMBOL_LENGTH as u64)
                    .and_then(|v| v.as_u64())
            {
                let so = sym_off as usize;
                let sl = sym_len as usize;
                if sl >= 4 && so.saturating_add(sl) <= bytes.len() {
                    doc_symbols = parse_doc_symbols(&bytes[so..so + sl]);
                }
            }
        }

        let max_possible_entries = if bytes.len() > KFX_HEADER_LEN {
            (bytes.len() - KFX_HEADER_LEN) / 24
        } else {
            0
        };
        let mut index_entries = Vec::with_capacity(index_count.min(max_possible_entries));

        for _ in 0..index_count {
            // `index_offset` is an attacker-controlled `u32` widened to `usize`, so
            // `curr_offset + 24` overflows — and panics under `overflow-checks = true` — on
            // 32-bit targets such as wasm32.
            if curr_offset.saturating_add(24) > bytes.len() {
                break;
            }
            let entity_id = u32::from_le_bytes([
                bytes[curr_offset],
                bytes[curr_offset + 1],
                bytes[curr_offset + 2],
                bytes[curr_offset + 3],
            ]);
            let type_id = u32::from_le_bytes([
                bytes[curr_offset + 4],
                bytes[curr_offset + 5],
                bytes[curr_offset + 6],
                bytes[curr_offset + 7],
            ]);
            let offset = u64::from_le_bytes([
                bytes[curr_offset + 8],
                bytes[curr_offset + 9],
                bytes[curr_offset + 10],
                bytes[curr_offset + 11],
                bytes[curr_offset + 12],
                bytes[curr_offset + 13],
                bytes[curr_offset + 14],
                bytes[curr_offset + 15],
            ]);
            let length = u64::from_le_bytes([
                bytes[curr_offset + 16],
                bytes[curr_offset + 17],
                bytes[curr_offset + 18],
                bytes[curr_offset + 19],
                bytes[curr_offset + 20],
                bytes[curr_offset + 21],
                bytes[curr_offset + 22],
                bytes[curr_offset + 23],
            ]);

            index_entries.push(KfxIndexEntry {
                entity_id,
                type_id,
                offset,
                length,
            });
            curr_offset += 24;
        }

        let payload_start = if is_ion_container {
            header_len
        } else {
            header_len.max(curr_offset)
        };
        let payload = if payload_start < bytes.len() {
            bytes[payload_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            version,
            is_ion_container,
            doc_symbols,
            index_entries,
            payload_start,
            payload,
        })
    }

    /// Build a binary KFX container stream from index entries and payload chunks.
    pub fn build(entries: &[KfxIndexEntry], payload: &[u8]) -> Vec<u8> {
        let mut buffer = Vec::new();
        let index_count = entries.len() as u32;
        let index_bytes_len = index_count as usize * 24;
        let header_len = (KFX_HEADER_LEN + index_bytes_len) as u32;

        // 1. Write 18-byte fixed CONT header
        buffer.extend_from_slice(KFX_MAGIC_CONT);
        let version: u16 = 2;
        buffer.extend_from_slice(&version.to_le_bytes());
        buffer.extend_from_slice(&header_len.to_le_bytes());
        let index_offset: u32 = KFX_HEADER_LEN as u32;
        buffer.extend_from_slice(&index_offset.to_le_bytes());
        buffer.extend_from_slice(&index_count.to_le_bytes());

        // 2. Write 24-byte index entries
        for entry in entries {
            buffer.extend_from_slice(&entry.entity_id.to_le_bytes());
            buffer.extend_from_slice(&entry.type_id.to_le_bytes());
            buffer.extend_from_slice(&entry.offset.to_le_bytes());
            buffer.extend_from_slice(&entry.length.to_le_bytes());
        }

        // 3. Write payload
        buffer.extend_from_slice(payload);

        // 4. Compute SHA-1 payload trailer
        let mut hasher = Sha1::new();
        hasher.update(&buffer);
        let hash_result = hasher.digest().bytes();
        buffer.extend_from_slice(&hash_result);

        buffer
    }
}

fn parse_doc_symbols(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(vals) = crate::kfx::ion::decode_stream(bytes) {
        for v in vals {
            // `$ion_symbol_table` field `symbols` has system symbol ID 7
            if let Some(list) = v.get_field(7).and_then(|f| f.as_list()) {
                for item in list {
                    if let Some(s) = item.as_str() {
                        out.push(s.to_string());
                    }
                }
            }
        }
    }
    out
}
