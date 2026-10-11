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
    ///
    /// Every offset here is read verbatim from the (untrusted) ZIP central directory, including
    /// the ZIP64 extra field, so the arithmetic is saturating: the release profile sets
    /// `overflow-checks = true`, meaning a bare `+` on a crafted `u64::MAX` field would panic.
    pub fn to_http_range_header(&self) -> (String, String) {
        // Range header spanning local header offset + entry payload.
        let end_byte = self
            .local_header_offset
            .saturating_add(30)
            .saturating_add(self.file_name.len() as u64)
            .saturating_add(self.extra_field_len.max(1024))
            .saturating_add(self.compressed_size);
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

        // Do not trust the declared entry count for pre-allocation: `find_eocd` scans backwards
        // for the first EOCD signature, which can be a false positive inside compressed data or a
        // comment, so a crafted 64-byte tail could otherwise force a 65 535-entry allocation
        // before a single entry is validated. Each entry needs at least 46 bytes, so the tail
        // length is a hard upper bound.
        let max_possible_entries = tail_bytes.len().saturating_sub(eocd_idx) / 46;
        let mut entries = Vec::with_capacity(entry_count.min(max_possible_entries));
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

            let mut final_comp_size = comp_size;
            let mut final_uncomp_size = uncomp_size;
            let mut final_offset = offset;

            if extra_len >= 4 {
                let extra_start = pos + 46 + name_len;
                let extra_end = (extra_start + extra_len).min(tail_bytes.len());
                let mut e_pos = extra_start;
                while e_pos + 4 <= extra_end {
                    let header_id = u16::from_le_bytes([tail_bytes[e_pos], tail_bytes[e_pos + 1]]);
                    let data_size =
                        u16::from_le_bytes([tail_bytes[e_pos + 2], tail_bytes[e_pos + 3]]) as usize;
                    let data_start = e_pos + 4;
                    let data_end = (data_start + data_size).min(extra_end);
                    if header_id == 0x0001 {
                        let mut d_pos = data_start;
                        if final_uncomp_size == 0xFFFFFFFF && d_pos + 8 <= data_end {
                            final_uncomp_size = u64::from_le_bytes(
                                tail_bytes[d_pos..d_pos + 8].try_into().unwrap_or([0; 8]),
                            );
                            d_pos += 8;
                        }
                        if final_comp_size == 0xFFFFFFFF && d_pos + 8 <= data_end {
                            final_comp_size = u64::from_le_bytes(
                                tail_bytes[d_pos..d_pos + 8].try_into().unwrap_or([0; 8]),
                            );
                            d_pos += 8;
                        }
                        if final_offset == 0xFFFFFFFF && d_pos + 8 <= data_end {
                            final_offset = u64::from_le_bytes(
                                tail_bytes[d_pos..d_pos + 8].try_into().unwrap_or([0; 8]),
                            );
                        }
                        break;
                    }
                    e_pos += 4 + data_size;
                }
            }

            let name_start = pos + 46;
            if name_start + name_len <= tail_bytes.len() {
                let file_name =
                    String::from_utf8_lossy(&tail_bytes[name_start..name_start + name_len])
                        .to_string();
                entries.push(ZipEntryLocation {
                    file_name,
                    local_header_offset: final_offset,
                    compressed_size: final_comp_size,
                    uncompressed_size: final_uncomp_size,
                    extra_field_len: extra_len as u64,
                });
            }

            pos += 46 + name_len + extra_len + comment_len;
        }

        Ok(entries)
    }
}
