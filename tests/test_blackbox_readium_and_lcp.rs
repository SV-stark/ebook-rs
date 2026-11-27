use ebook_rs::TxtBook;
use ebook_rs::lcp::{LcpDecryptor, LcpLicense};

#[test]
fn test_blackbox_readium_webpub_manifest_export() {
    let md = "# WebPub Test\n\nChapter content for webpub manifest export.";
    let book = TxtBook::parse(md.as_bytes(), "WebPub Book", true).unwrap();

    let manifest = book.to_webpub_manifest();
    assert_eq!(manifest.metadata.title, "WebPub Test");
    assert!(!manifest.reading_order.is_empty());
}

#[test]
fn test_blackbox_readium_unified_locator_model() {
    let md = "# Chapter 1\nSection text content for locator testing.";
    let book = TxtBook::parse(md.as_bytes(), "Locator Book", true).unwrap();

    let locator = book.to_readium_locator(0, 5).unwrap();
    assert_eq!(locator.type_, "application/xhtml+xml");
    assert!(locator.locations.cfi.is_some());
    assert!(locator.locations.position.is_some());
    assert!(locator.text.is_some());
}

#[test]
fn test_blackbox_readium_lcp_license_parsing() {
    let json_license = r#"{
        "id": "lic-999",
        "issued": "2026-01-01T00:00:00Z",
        "provider": "http://example.com/lcp",
        "user": { "id": "usr-1" },
        "encryption": {
            "profile": "http://readium.org/lcp/basic-profile",
            "user_key": { "algorithm": "http://www.w3.org/2001/04/xmlenc#sha256" }
        },
        "links": [
            { "rel": "publication", "href": "http://example.com/book.epub" }
        ]
    }"#;

    let license = LcpLicense::parse(json_license).expect("LCP JSON parse error");
    assert_eq!(license.id, "lic-999");
    assert_eq!(license.provider, "http://example.com/lcp");
    assert_eq!(
        license.encryption.as_ref().unwrap().profile,
        "http://readium.org/lcp/basic-profile"
    );

    // Empty passphrase should return error
    let res = LcpDecryptor::decrypt_bytes(b"encrypted_data", "", &license);
    assert!(res.is_err());
}

#[test]
fn test_blackbox_readium_lcp_expiration_date_comparison() {
    let json_license = r#"{
        "id": "lic-expiry",
        "provider": "http://example.com/lcp",
        "rights": {
            "end": "2026-12-31"
        }
    }"#;

    let license = LcpLicense::parse(json_license).expect("LCP JSON parse error");

    // During end day 2026-12-31, license must NOT be expired
    assert!(!license.is_expired("2026-12-31T10:00:00Z"));
    assert!(!license.is_expired("2026-12-31T23:59:59Z"));

    // On 2027-01-01, license MUST be expired
    assert!(license.is_expired("2027-01-01T00:00:00Z"));
}

#[test]
fn test_blackbox_font_deobfuscation() {
    use ebook_rs::deobfuscate::FontDeobfuscator;

    let enc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <EncryptedData>
    <EncryptionMethod Algorithm="http://www.idpf.org/2008/embedding"/>
    <CipherData>
      <CipherReference URI="fonts/font1.otf"/>
    </CipherData>
  </EncryptedData>
  <EncryptedData>
    <EncryptionMethod Algorithm="http://ns.adobe.com/pdf/enc#RC"/>
    <CipherData>
      <CipherReference URI="fonts/font2.otf"/>
    </CipherData>
  </EncryptedData>
</encryption>"#;

    let deobfuscator = FontDeobfuscator::parse_encryption_xml(enc_xml);
    assert!(deobfuscator.is_encrypted("fonts/font1.otf"));
    assert!(deobfuscator.is_encrypted("fonts/font2.otf"));

    let mut font1_bytes = vec![0xAB; 2048];
    let orig1 = font1_bytes.clone();
    let key = "urn:uuid:12345678-1234-5678-1234-567812345678";

    // 1. IDPF algorithm in-place deobfuscation
    deobfuscator.deobfuscate("fonts/font1.otf", &mut font1_bytes, key);
    assert_ne!(&font1_bytes[..1040], &orig1[..1040]);
    assert_eq!(&font1_bytes[1040..], &orig1[1040..]);

    // 2. Adobe algorithm in-place deobfuscation
    let mut font2_bytes = vec![0xCD; 2048];
    let orig2 = font2_bytes.clone();
    deobfuscator.deobfuscate("fonts/font2.otf", &mut font2_bytes, key);
    assert_ne!(&font2_bytes[..1024], &orig2[..1024]);
    assert_eq!(&font2_bytes[1024..], &orig2[1024..]);
}

#[test]
fn test_lcp_official_encryption_content_key_sibling_and_webpub_manifest_enhancements() {
    use base64::Engine;
    use ebook_rs::{Book, generate_sample_epub};

    // 1. Test `encryption.content_key` sibling path (official LCP spec)
    let passphrase = "secret-passphrase";
    let invalid_b64 = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 48]);
    let json_license = format!(
        r#"{{
            "id": "lic-sibling",
            "provider": "http://example.com/lcp",
            "encryption": {{
                "profile": "http://readium.org/lcp/basic-profile",
                "user_key": {{ "algorithm": "http://www.w3.org/2001/04/xmlenc#sha256" }},
                "content_key": {{ "encrypted_value": "{}" }}
            }}
        }}"#,
        invalid_b64
    );
    let license = LcpLicense::parse(&json_license).unwrap();
    assert!(license.encryption.as_ref().unwrap().content_key.is_some());
    // Attempting decryption with wrong padding must return content_key decryption error, not succeed
    let encrypted = vec![0u8; 32];
    let err = LcpDecryptor::decrypt_bytes(&encrypted, passphrase, &license).unwrap_err();
    assert!(err.contains("content key"));

    // 2. Test WebPub Manifest: identifier, description, TOC-derived readingOrder titles, resources
    let epub_bytes = generate_sample_epub().unwrap();
    let book = Book::from_bytes(&epub_bytes).unwrap();
    let manifest = book.to_webpub_manifest();
    assert!(!manifest.resources.is_empty());
    assert!(manifest.metadata.identifier.is_some());
    assert!(!manifest.reading_order.is_empty());

    // 3. Test ReadiumLocator `fragments` alongside `fragment`
    let loc = book.to_readium_locator(0, 0).unwrap();
    if let Some(ref frag) = loc.locations.fragment {
        assert_eq!(loc.locations.fragments, vec![frag.clone()]);
    }
}
