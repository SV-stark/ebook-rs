use ahash::AHashMap;

/// IDPF & Adobe Font De-obfuscation Engine.
#[derive(Debug, Clone)]
pub struct FontDeobfuscator {
    encrypted_fonts: AHashMap<String, String>, // path -> algorithm URI
}

impl Default for FontDeobfuscator {
    fn default() -> Self {
        Self::new()
    }
}

impl FontDeobfuscator {
    pub fn new() -> Self {
        Self {
            encrypted_fonts: AHashMap::new(),
        }
    }

    /// Parse `META-INF/encryption.xml` if present in archive.
    pub fn parse_encryption_xml(xml_content: &str) -> Self {
        let mut encrypted_fonts = AHashMap::new();
        if let Ok(doc) = roxmltree::Document::parse(xml_content) {
            for node in doc.descendants() {
                if node.has_tag_name("EncryptedData") {
                    let mut algo = String::new();
                    let mut uri = String::new();

                    for child in node.children() {
                        if child.has_tag_name("EncryptionMethod") {
                            if let Some(a) = child.attribute("Algorithm") {
                                algo = a.to_string();
                            }
                        } else if child.has_tag_name("CipherData") {
                            for sub in child.children() {
                                if sub.has_tag_name("CipherReference") {
                                    if let Some(u) = sub.attribute("URI") {
                                        uri = u.to_string();
                                    }
                                }
                            }
                        }
                    }

                    if !uri.is_empty() && !algo.is_empty() {
                        encrypted_fonts.insert(uri.to_lowercase(), algo);
                    }
                }
            }
        }

        Self { encrypted_fonts }
    }

    /// Check if a font path is encrypted.
    pub fn is_encrypted(&self, font_path: &str) -> bool {
        self.encrypted_fonts.contains_key(&font_path.to_lowercase())
    }

    /// De-obfuscate font bytes using book identifier key.
    pub fn deobfuscate(&self, font_path: &str, font_bytes: &mut [u8], identifier: &str) {
        if let Some(algo) = self.encrypted_fonts.get(&font_path.to_lowercase()) {
            if algo.contains("2008/embedding") || algo.contains("idpf") {
                deobfuscate_idpf(font_bytes, identifier);
            } else if algo.contains("2006/enc-font") || algo.contains("adobe") {
                deobfuscate_adobe(font_bytes, identifier);
            }
        }
    }
}

/// IDPF Font De-obfuscation: XOR first 1040 bytes with SHA-1 of EPUB identifier.
pub fn deobfuscate_idpf(font_bytes: &mut [u8], identifier: &str) {
    let clean_id: String = identifier
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    let key = sha1(clean_id.as_bytes());
    let len = font_bytes.len().min(1040);
    for i in 0..len {
        font_bytes[i] ^= key[i % 20];
    }
}

/// Adobe Font De-obfuscation: XOR first 1024 bytes with GUID key of EPUB identifier.
pub fn deobfuscate_adobe(font_bytes: &mut [u8], identifier: &str) {
    let key = parse_adobe_guid_key(identifier);
    let len = font_bytes.len().min(1024);
    for i in 0..len {
        font_bytes[i] ^= key[i % 16];
    }
}

fn parse_adobe_guid_key(identifier: &str) -> [u8; 16] {
    let clean = identifier.trim();
    let without_prefix = clean
        .strip_prefix("urn:uuid:")
        .or_else(|| clean.strip_prefix("URN:UUID:"))
        .or_else(|| clean.strip_prefix("urn:guid:"))
        .or_else(|| clean.strip_prefix("URN:GUID:"))
        .unwrap_or(clean);

    let hex_str: String = without_prefix
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    let mut key = [0u8; 16];
    let bytes = hex_str.as_bytes();
    for i in 0..16 {
        if i * 2 + 1 < bytes.len() {
            let s = std::str::from_utf8(&bytes[i * 2..i * 2 + 2]).unwrap_or("00");
            key[i] = u8::from_str_radix(s, 16).unwrap_or(0);
        }
    }
    key
}

/// Minimal pure Rust SHA-1 hash function for IDPF font de-obfuscation key generation.
#[allow(clippy::needless_range_loop)]
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let ml = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}
