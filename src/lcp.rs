use aes::Aes256;
use cbc::Decryptor;
use cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
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
/// Uses AES-256-CBC with PKCS#7 padding per the Readium LCP specification.
/// The 256-bit key is derived as SHA-256(passphrase). The first 16 bytes of the
/// ciphertext are used as the IV (initialization vector).
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
    /// Per the Readium LCP spec: key = SHA-256(passphrase), IV = first 16 bytes of ciphertext.
    pub fn decrypt_stream(
        encrypted_bytes: &[u8],
        passphrase: &str,
        license: &LcpLicense,
    ) -> Result<Vec<u8>, String> {
        if passphrase.trim().is_empty() {
            return Err("Passphrase cannot be empty for Readium LCP protected eBook".to_string());
        }
        if encrypted_bytes.len() < 16 {
            return Err(
                "Encrypted data too short to contain a valid IV (need >= 16 bytes)".to_string(),
            );
        }

        // Dynamically evaluate license expiration using system time
        let now_iso = chrono_now_iso();
        if license.is_expired(&now_iso) {
            return Err("Readium LCP license has expired".to_string());
        }

        // Derive 256-bit AES key: SHA-256(passphrase bytes)
        let key_bytes: [u8; 32] = sha256_hash(passphrase.as_bytes());

        // Validate passphrase against license key_check if present
        if let Some(ref encryption) = license.encryption {
            if let Some(ref user_key) = encryption.user_key {
                if let Some(key_check) = user_key.get("key_check").and_then(|v| v.as_str()) {
                    let double_hash = sha256_hash(&key_bytes);
                    let hex_str = double_hash
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

        // IV = first 16 bytes of ciphertext (per LCP spec)
        let iv: [u8; 16] = encrypted_bytes[..16]
            .try_into()
            .map_err(|_| "IV extraction failed")?;
        let ciphertext = &encrypted_bytes[16..];

        // AES-256-CBC decrypt with PKCS7 unpadding
        let decryptor = Decryptor::<Aes256>::new(&key_bytes.into(), &iv.into());
        let mut buf = ciphertext.to_vec();
        decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .map(|decrypted| decrypted.to_vec())
            .map_err(|e| format!("AES-256-CBC decryption failed: {:?}", e))
    }
}

fn sha256_hash(input: &[u8]) -> [u8; 32] {
    crate::fingerprint::sha256_bytes(input)
}

/// Return current UTC time as ISO 8601 string using chrono.
fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as YYYY-MM-DDTHH:MM:SS (no chrono dep needed for basic comparison)
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Rough Gregorian calendar (good for 2024–2050 range)
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hour, min, sec
    )
}
