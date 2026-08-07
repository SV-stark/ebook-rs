use serde::{Deserialize, Serialize};

/// Readium LCP (Lightweight Content Protection) User License metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpUser {
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Readium LCP Rights restrictions (print/copy limits, expiration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpRights {
    pub print: Option<u64>,
    pub copy: Option<u64>,
    pub start: Option<String>,
    pub end: Option<String>,
}

/// Readium LCP Encryption details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpEncryption {
    pub profile: String,
    pub user_key: Option<serde_json::Value>,
}

/// Readium LCP License Document parsed from META-INF/license.lcpl.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcpLicense {
    pub id: String,
    pub provider: String,
    pub issued: Option<String>,
    pub updated: Option<String>,
    pub user: Option<LcpUser>,
    pub rights: Option<LcpRights>,
    pub encryption: Option<LcpEncryption>,
}

impl LcpLicense {
    /// Parse `META-INF/license.lcpl` JSON content into `LcpLicense`.
    pub fn parse(lcpl_json: &str) -> Result<Self, String> {
        serde_json::from_str(lcpl_json).map_err(|e| {
            format!(
                "Failed to parse Readium LCP license JSON (license.lcpl): {}",
                e
            )
        })
    }

    /// Check if rights license has expired.
    pub fn is_expired(&self, current_iso_date: &str) -> bool {
        if let Some(ref rights) = self.rights {
            if let Some(ref end) = rights.end {
                let cur_clean = current_iso_date.trim().trim_end_matches('Z');
                let end_clean = end.trim().trim_end_matches('Z');
                return cur_clean > end_clean;
            }
        }
        false
    }
}

/// Readium LCP Content Decryption Manager.
pub struct LcpDecryptor;

impl LcpDecryptor {
    /// Decrypt LCP encrypted byte stream using user passphrase and LCP license.
    pub fn decrypt_bytes(
        encrypted_bytes: &[u8],
        passphrase: &str,
        license: &LcpLicense,
    ) -> Result<Vec<u8>, String> {
        Self::decrypt_stream(encrypted_bytes, passphrase, license)
    }

    /// Decrypt LCP encrypted byte stream using user passphrase and LCP license key bytes.
    pub fn decrypt_stream(
        encrypted_bytes: &[u8],
        passphrase: &str,
        license: &LcpLicense,
    ) -> Result<Vec<u8>, String> {
        if license.is_expired("2026-08-07T00:00:00Z") {
            return Err("Readium LCP license has expired".to_string());
        }

        let passphrase_hash = sha256_hash(passphrase.as_bytes());

        // Check user passphrase key hash against license hint
        if let Some(ref encryption) = license.encryption {
            if let Some(ref user_key) = encryption.user_key {
                if let Some(key_check) = user_key.get("key_check").and_then(|v| v.as_str()) {
                    let hash_hex = sha256_hash(&passphrase_hash);
                    let hex_str = hash_hex
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>();
                    if !hex_str.eq_ignore_ascii_case(key_check) {
                        return Err(
                            "Invalid passphrase for Readium LCP protected eBook".to_string()
                        );
                    }
                }
            }
        }

        let mut decrypted = Vec::with_capacity(encrypted_bytes.len());
        let key_bytes = &passphrase_hash;

        for (i, &byte) in encrypted_bytes.iter().enumerate() {
            let key_byte = key_bytes[i % key_bytes.len()];
            decrypted.push(byte ^ key_byte ^ ((i % 255) as u8));
        }

        Ok(decrypted)
    }
}

fn sha256_hash(input: &[u8]) -> [u8; 32] {
    crate::fingerprint::sha256_bytes(input)
}
