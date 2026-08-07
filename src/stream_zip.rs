use serde::{Deserialize, Serialize};

/// Location index of a file entry inside a ZIP archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipEntryLocation {
    pub file_name: String,
    pub local_header_offset: u64,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    #[serde(default)]
    pub extra_field_len: u64,
}

impl ZipEntryLocation {
    /// Generate HTTP Range header for fetching this entry over HTTP.
    pub fn to_http_range_header(&self) -> (String, String) {
        // Range header spanning local header offset + entry payload
        let end_byte = self.local_header_offset
            + 30
            + self.file_name.len() as u64
            + self.extra_field_len.max(1024)
            + self.compressed_size;
        (
            "Range".to_string(),
            format!("bytes={}-{}", self.local_header_offset, end_byte),
        )
    }
}

/// Remote ZIP Central Directory Header Streamer.
#[derive(Debug, Clone)]
pub struct ZipHeaderReader;

impl ZipHeaderReader {
    /// Scan tail byte slice (last ~64KB of ZIP archive) for End of Central Directory (EOCD) signature `PK\x05\x06`.
    pub fn find_eocd(bytes: &[u8]) -> Option<usize> {
        if bytes.len() < 22 {
            return None;
        }
        let eocd_sig = b"PK\x05\x06";
        let mut idx = bytes.len() - 22;
        loop {
            if &bytes[idx..idx + 4] == eocd_sig {
                return Some(idx);
            }
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        None
    }

    /// Parse End of Central Directory record and Central Directory entries from tail bytes.
    pub fn parse_central_directory(tail_bytes: &[u8]) -> Result<Vec<ZipEntryLocation>, String> {
        let eocd_idx = Self::find_eocd(tail_bytes).ok_or_else(|| {
            "EOCD record (PK\\x05\\x06) signature not found in tail bytes".to_string()
        })?;

        if tail_bytes.len() < eocd_idx + 22 {
            return Err("Truncated EOCD header".to_string());
        }

        let entry_count =
            u16::from_le_bytes([tail_bytes[eocd_idx + 10], tail_bytes[eocd_idx + 11]]) as usize;
        let cd_size = u32::from_le_bytes([
            tail_bytes[eocd_idx + 12],
            tail_bytes[eocd_idx + 13],
            tail_bytes[eocd_idx + 14],
            tail_bytes[eocd_idx + 15],
        ]) as usize;

        let mut entries = Vec::with_capacity(entry_count);
        let cd_start = eocd_idx.saturating_sub(cd_size);

        let mut pos = cd_start;
        let cd_sig = b"PK\x01\x02";

        while pos + 46 <= eocd_idx {
            if &tail_bytes[pos..pos + 4] != cd_sig {
                pos += 1;
                continue;
            }

            let comp_size = u32::from_le_bytes([
                tail_bytes[pos + 20],
                tail_bytes[pos + 21],
                tail_bytes[pos + 22],
                tail_bytes[pos + 23],
            ]) as u64;

            let uncomp_size = u32::from_le_bytes([
                tail_bytes[pos + 24],
                tail_bytes[pos + 25],
                tail_bytes[pos + 26],
                tail_bytes[pos + 27],
            ]) as u64;

            let name_len =
                u16::from_le_bytes([tail_bytes[pos + 28], tail_bytes[pos + 29]]) as usize;
            let extra_len =
                u16::from_le_bytes([tail_bytes[pos + 30], tail_bytes[pos + 31]]) as usize;
            let comment_len =
                u16::from_le_bytes([tail_bytes[pos + 32], tail_bytes[pos + 33]]) as usize;

            let offset = u32::from_le_bytes([
                tail_bytes[pos + 42],
                tail_bytes[pos + 43],
                tail_bytes[pos + 44],
                tail_bytes[pos + 45],
            ]) as u64;

            let name_start = pos + 46;
            if name_start + name_len <= tail_bytes.len() {
                let file_name =
                    String::from_utf8_lossy(&tail_bytes[name_start..name_start + name_len])
                        .to_string();
                entries.push(ZipEntryLocation {
                    file_name,
                    local_header_offset: offset,
                    compressed_size: comp_size,
                    uncompressed_size: uncomp_size,
                    extra_field_len: extra_len as u64,
                });
            }

            pos += 46 + name_len + extra_len + comment_len;
        }

        Ok(entries)
    }
}
