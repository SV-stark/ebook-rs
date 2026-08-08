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
    pub index_entries: Vec<KfxIndexEntry>,
    pub payload: Vec<u8>,
}

impl KfxContainer {
    /// Detect if bytes start with valid KFX magic.
    pub fn is_kfx(bytes: &[u8]) -> bool {
        bytes.len() >= 4 && &bytes[0..4] == KFX_MAGIC_CONT
    }

    /// Parse raw container bytes into KfxContainer.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if !Self::is_kfx(bytes) {
            return Err("Invalid KFX container magic bytes".to_string());
        }
        if bytes.len() < KFX_HEADER_LEN {
            return Err("Truncated KFX container header".to_string());
        }

        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let header_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        let index_offset =
            u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let index_count = u32::from_le_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]) as usize;

        let mut index_entries = Vec::with_capacity(index_count);
        let mut curr_offset = if index_offset > 0 {
            index_offset
        } else {
            KFX_HEADER_LEN
        };

        for _ in 0..index_count {
            if curr_offset + 24 > bytes.len() {
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

        let payload_start = header_len.max(curr_offset);
        let payload = if payload_start < bytes.len() {
            bytes[payload_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            version,
            index_entries,
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
