use aes::Aes256;
use base64::Engine;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeDecrypt, KeyIvInit};
use serde::{Deserialize, Serialize};

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    let bytes_input = s.as_bytes();
    if !bytes_input.len().is_multiple_of(2) {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(bytes_input.len() / 2);
    for chunk in bytes_input.as_chunks::<2>().0 {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        bytes.push((hi << 4) | lo);
    }
    Ok(bytes)
}

fn hex_nibble(b: u8) -> Result<u8, ()> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(()),
    }
}

/// Readium LCP (Lightweight Content Protection) User License metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LcpUser {
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Readium LCP Rights restrictions (print/copy limits, expiration).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
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
                let cur_ts = parse_iso_date_to_seconds(current_iso_date, false);
                let end_ts = parse_iso_date_to_seconds(end, true);
                if let (Some(cur), Some(end)) = (cur_ts, end_ts) {
                    return cur > end;
                }
                let cur_norm = normalize_iso_timestamp(current_iso_date, false);
                let end_norm = normalize_iso_timestamp(end, true);
                return cur_norm > end_norm;
            }
        }
        false
    }
}

fn parse_iso_date_to_seconds(s: &str, is_end_of_day: bool) -> Option<i64> {
    let clean = s.trim();
    if clean.len() >= 10 {
        let year: i32 = clean[0..4].parse().ok()?;
        let month: u32 = clean[5..7].parse().ok()?;
        let day: u32 = clean[8..10].parse().ok()?;
        let (hour, min, sec, offset_minutes) = if clean.len() > 10 && clean.as_bytes()[10] == b'T' {
            let time_part = &clean[11..];
            let (hms, tz) = if let Some(pos) = time_part.find(&['+', '-', 'Z'][..]) {
                (&time_part[..pos], &time_part[pos..])
            } else {
                (time_part, "")
            };
            let parts: Vec<&str> = hms.split(':').collect();
            let h = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
            let m = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            let s = parts
                .get(2)
                .and_then(|p| p.split('.').next().unwrap_or("").parse().ok())
                .unwrap_or(0);

            let tz_off = if tz.starts_with('+') || tz.starts_with('-') {
                let sign = if tz.starts_with('+') { 1 } else { -1 };
                let tz_clean = &tz[1..];
                if let Some((th, tm)) = tz_clean.split_once(':') {
                    let h_val: i64 = th.parse().unwrap_or(0);
                    let m_val: i64 = tm.parse().unwrap_or(0);
                    sign * (h_val * 60 + m_val)
                } else if tz_clean.len() == 2 {
                    let h_val: i64 = tz_clean.parse().unwrap_or(0);
                    sign * (h_val * 60)
                } else {
                    0
                }
            } else {
                0
            };
            (h, m, s, tz_off)
        } else if is_end_of_day {
            (23, 59, 59, 0)
        } else {
            (0, 0, 0, 0)
        };

        let days = days_from_civil(year, month, day);
        let secs = days * 86400 + (hour as i64 * 3600) + (min as i64 * 60) + (sec as i64)
            - (offset_minutes * 60);
        Some(secs)
    } else {
        None
    }
}

fn days_from_civil(mut y: i32, m: u32, d: u32) -> i64 {
    y -= (m <= 2) as i32;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era as i64 * 146097 + doe as i64) - 719468
}

fn normalize_iso_timestamp(s: &str, is_end_of_day: bool) -> String {
    let clean = s.trim().trim_end_matches('Z');
    if clean.len() == 10 {
        if is_end_of_day {
            format!("{}T23:59:59", clean)
        } else {
            format!("{}T00:00:00", clean)
        }
    } else {
        clean.to_string()
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

        // Derive 256-bit AES user key: SHA-256(passphrase bytes)
        let user_key_bytes: [u8; 32] = sha256_hash(passphrase.as_bytes());

        // Validate passphrase against license key_check if present
        if let Some(ref encryption) = license.encryption {
            if let Some(ref user_key) = encryption.user_key {
                if let Some(key_check) = user_key.get("key_check").and_then(|v| v.as_str()) {
                    let mut valid = false;
                    // Check if key_check is encrypted license id (hex or base64)
                    if let Ok(kc_bytes) = hex_decode(key_check).or_else(|_| {
                        base64::engine::general_purpose::STANDARD
                            .decode(key_check)
                            .map_err(|_| ())
                    }) {
                        if kc_bytes.len() >= 16 {
                            let kc_iv: [u8; 16] = kc_bytes[..16].try_into().unwrap_or([0u8; 16]);
                            let kc_cipher = &kc_bytes[16..];
                            let decryptor = cbc::Decryptor::<Aes256>::new(
                                &user_key_bytes.into(),
                                &kc_iv.into(),
                            );
                            let mut buf = kc_cipher.to_vec();
                            if let Ok(decrypted) = decryptor.decrypt_padded::<Pkcs7>(&mut buf) {
                                if let Ok(s) = std::str::from_utf8(decrypted) {
                                    if s == license.id {
                                        valid = true;
                                    }
                                }
                            }
                        }
                    }
                    if !valid {
                        return Err(
                            "Invalid passphrase for Readium LCP protected eBook".to_string()
                        );
                    }
                }
            }
        }

        // Determine content encryption key (CEK): either decrypted from content_key or direct user_key
        let mut final_aes_key = user_key_bytes;
        if let Some(ref encryption) = license.encryption {
            if let Some(content_key) = encryption
                .user_key
                .as_ref()
                .and_then(|uk| uk.get("content_key"))
            {
                if let Some(enc_val) = content_key.get("encrypted_value").and_then(|v| v.as_str()) {
                    if let Ok(enc_bytes) = base64::engine::general_purpose::STANDARD.decode(enc_val)
                    {
                        if enc_bytes.len() >= 32 {
                            let ck_iv: [u8; 16] = enc_bytes[..16].try_into().unwrap_or([0u8; 16]);
                            let ck_cipher = &enc_bytes[16..];
                            let decryptor = cbc::Decryptor::<Aes256>::new(
                                &user_key_bytes.into(),
                                &ck_iv.into(),
                            );
                            let mut buf = ck_cipher.to_vec();
                            if let Ok(decrypted) = decryptor.decrypt_padded::<Pkcs7>(&mut buf) {
                                if decrypted.len() == 32 {
                                    final_aes_key.copy_from_slice(decrypted);
                                }
                            }
                        }
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
        let decryptor = cbc::Decryptor::<Aes256>::new(&final_aes_key.into(), &iv.into());
        let mut buf = ciphertext.to_vec();
        decryptor
            .decrypt_padded::<Pkcs7>(&mut buf)
            .map(|decrypted| decrypted.to_vec())
            .map_err(|e| format!("AES-256-CBC decryption failed: {:?}", e))
    }
}

fn sha256_hash(input: &[u8]) -> [u8; 32] {
    crate::fingerprint::sha256_bytes(input)
}

/// Return current UTC time as ISO 8601 string using real Gregorian leap-year calendar.
fn chrono_now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sec = (secs % 60) as u32;
    let min = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;
    let mut days = (secs / 86400) as i64;

    let mut year = 1970;
    loop {
        let leap = is_leap_year(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_year(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &d in &month_days {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    let day = (days + 1) as u32;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hour, min, sec
    )
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
