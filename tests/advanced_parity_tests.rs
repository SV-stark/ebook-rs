use ebook_rs::{
    deobfuscate::{deobfuscate_adobe, deobfuscate_idpf, FontDeobfuscator},
    generate_sample_epub,
    nav::{parse_landmarks, parse_page_list},
    Book,
};

#[test]
fn test_epub3_landmarks_and_page_list_parsing() {
    let html = r#"
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
            <body>
                <nav epub:type="landmarks">
                    <h2>Landmarks</h2>
                    <ol>
                        <li><a epub:type="cover" href="cover.xhtml">Cover</a></li>
                        <li><a epub:type="toc" href="toc.xhtml">Table of Contents</a></li>
                        <li><a epub:type="bodymatter" href="chapter1.xhtml">Start Reading</a></li>
                    </ol>
                </nav>
                <nav epub:type="page-list">
                    <h2>Print Page List</h2>
                    <ol>
                        <li><a href="chap1.xhtml#page1">1</a></li>
                        <li><a href="chap1.xhtml#page2">2</a></li>
                    </ol>
                </nav>
            </body>
        </html>
    "#;

    let landmarks = parse_landmarks(html);
    assert_eq!(landmarks.len(), 3);
    assert_eq!(landmarks[0].epub_type, "cover");
    assert_eq!(landmarks[0].href, "cover.xhtml");

    let page_list = parse_page_list(html);
    assert_eq!(page_list.len(), 2);
    assert_eq!(page_list[0].page, "1");
    assert_eq!(page_list[0].href, "chap1.xhtml#page1");
}

#[test]
fn test_before_display_transformation_hooks() {
    let bytes = generate_sample_epub().unwrap();
    let mut book = Book::from_bytes(&bytes).unwrap();

    // Register a pre-display hook that injects custom watermarks into section HTML
    book.register_before_display_hook(|html, path| {
        html.push_str(&format!(
            "<div class=\"watermark\">Processed: {}</div>",
            path
        ));
    });

    let sec0 = book.get_section(0).expect("Section 0");
    assert!(sec0.processed_html.contains("Processed: OEBPS/ch1.xhtml"));
}

#[test]
fn test_font_deobfuscation_idpf_and_adobe() {
    let identifier = "urn:uuid:9780694014545";
    let original = [0x4F, 0x74, 0x74, 0x6F, 0x00, 0x01, 0x00, 0x00];

    // Test IDPF XOR de-obfuscation roundtrip
    let mut encrypted = original;
    deobfuscate_idpf(&mut encrypted, identifier);
    assert_ne!(encrypted, original); // Obfuscated

    deobfuscate_idpf(&mut encrypted, identifier);
    assert_eq!(encrypted, original); // De-obfuscated back to original!

    // Test Adobe XOR de-obfuscation roundtrip
    let mut encrypted_adobe = original;
    deobfuscate_adobe(&mut encrypted_adobe, identifier);
    assert_ne!(encrypted_adobe, original);

    deobfuscate_adobe(&mut encrypted_adobe, identifier);
    assert_eq!(encrypted_adobe, original);

    // Test encryption.xml parsing
    let xml = r#"
        <encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
            <EncryptedData>
                <EncryptionMethod Algorithm="http://www.idpf.org/2008/embedding"/>
                <CipherData>
                    <CipherReference URI="OEBPS/Fonts/custom.otf"/>
                </CipherData>
            </EncryptedData>
        </encryption>
    "#;
    let deobf = FontDeobfuscator::parse_encryption_xml(xml);
    assert!(deobf.is_encrypted("OEBPS/Fonts/custom.otf"));
}
