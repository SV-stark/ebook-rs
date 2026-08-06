// Readium LCP DRM, Locator Model, and Search API integration tests (v0.6.0 rebuild)
use ebook_rs::Book;
use ebook_rs::lcp::{LcpDecryptor, LcpLicense};
use ebook_rs::search::SearchEngine;

#[test]
fn test_readium_lcp_license_parsing_and_decryption() {
    let lcpl_json = r#"{
        "id": "urn:uuid:12345-6789",
        "provider": "http://readium.org",
        "issued": "2026-08-01T00:00:00Z",
        "user": {
            "id": "usr_99",
            "email": "reader@example.com",
            "name": "Rust Reader"
        },
        "rights": {
            "print": 20,
            "copy": 50,
            "end": "2026-12-31T23:59:59Z"
        },
        "encryption": {
            "profile": "http://readium.org/lcp/basic-profile"
        }
    }"#;

    let license = LcpLicense::parse(lcpl_json).expect("Should parse license.lcpl");
    assert_eq!(license.id, "urn:uuid:12345-6789");
    assert_eq!(
        license.user.as_ref().unwrap().name.as_deref(),
        Some("Rust Reader")
    );
    assert!(!license.is_expired("2026-08-06T12:00:00Z"));
    assert!(license.is_expired("2027-01-01T00:00:00Z"));

    let encrypted_data = b"ENCRYPTED_PDF_OR_EPUB_CONTENT_DATA";
    let decrypted = LcpDecryptor::decrypt_bytes(encrypted_data, "user_secret_passphrase", &license)
        .expect("Should decrypt LCP bytes");
    assert_eq!(decrypted.len(), encrypted_data.len());
}

#[test]
fn test_readium_unified_locator_model() {
    let epub_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(epub_path).expect("Should parse EPUB3");

    let locator = book
        .to_readium_locator(0, 50)
        .expect("Should generate ReadiumLocator");
    assert!(!locator.href.is_empty());
    assert_eq!(locator.type_, "application/xhtml+xml");
    assert!(locator.locations.cfi.is_some());
    assert!(locator.locations.position.is_some());
    assert!(locator.locations.progression >= 0.0 && locator.locations.progression <= 1.0);
    assert!(locator.locations.total_progression >= 0.0);
    assert!(locator.text.is_some());
}

#[test]
fn test_readium_search_web_service_api() {
    let epub_path = "samples/Alice in Wonderland - Lewis Carroll EPUB3.epub";
    let book = Book::from_file(epub_path).expect("Should parse EPUB3");

    let results = book.search("Alice");
    assert!(!results.is_empty());

    let json_search = SearchEngine::to_readium_search_json(&results, "Alice")
        .expect("Should generate Readium Search JSON");

    assert!(json_search.contains("http://readium.org/webpub-manifest/context.jsonld"));
    assert!(json_search.contains("numberOfResults"));
    assert!(json_search.contains("locators"));
    assert!(json_search.contains("application/xhtml+xml"));
}
