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
