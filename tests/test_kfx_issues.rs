use ebook_rs::kfx::container::KfxContainer;
use ebook_rs::kfx::ion::{IonValue, decode_stream};
use ebook_rs::kfx::symbols::*;
use ebook_rs::metadata::PageProgressionDirection;
use ebook_rs::{Book, KfxBook, TxtBook, UniversalKfxExporter};
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn build_test_epub(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (path, data) in files {
            zip.start_file(*path, deflated).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

fn decode_enty(payload: &[u8]) -> IonValue {
    let hdr_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;
    decode_stream(&payload[hdr_len..]).unwrap().remove(0)
}

fn extract_kfx_title_meta(kfx_bytes: &[u8]) -> Vec<(String, String)> {
    let c = KfxContainer::parse(kfx_bytes).unwrap();
    for entry in &c.index_entries {
        if entry.type_id == SYM_BOOK_METADATA {
            let val = decode_enty(c.get_entry_payload(entry).unwrap());
            let cats = val.get_field(491).unwrap().as_list().unwrap();
            for cat in cats {
                if cat.get_field(495).and_then(|v| v.as_str()) == Some("kindle_title_metadata") {
                    let items = cat.get_field(258).unwrap().as_list().unwrap();
                    return items
                        .iter()
                        .filter_map(|item| {
                            let k = item.get_field(492)?.as_str()?.to_string();
                            let v = item.get_field(307)?.as_str()?.to_string();
                            Some((k, v))
                        })
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

// 1x1 valid PNG
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[test]
fn issue_24_unique_deterministic_publication_identifiers() {
    let book_a = TxtBook::parse(b"Alpha content here.", "Alpha Book", false).unwrap();
    let book_b = TxtBook::parse(b"Beta content here.", "Beta Book", false).unwrap();

    let kfx_a1 = UniversalKfxExporter::export(&book_a).unwrap();
    let kfx_a2 = UniversalKfxExporter::export(&book_a).unwrap();
    let kfx_b = UniversalKfxExporter::export(&book_b).unwrap();

    // Deterministic for identical book
    assert_eq!(kfx_a1, kfx_a2);

    let meta_a = extract_kfx_title_meta(&kfx_a1);
    let meta_b = extract_kfx_title_meta(&kfx_b);

    let get = |m: &[(String, String)], key: &str| -> String {
        m.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap()
    };

    for key in ["ASIN", "asset_id", "book_id", "content_id"] {
        assert_ne!(
            get(&meta_a, key),
            get(&meta_b, key),
            "Identifier {key} should differ across distinct books"
        );
    }
    assert_eq!(get(&meta_a, "ASIN"), get(&meta_a, "content_id"));
    assert!(get(&meta_a, "asset_id").starts_with("CR!"));
}

#[test]
fn issue_25_26_27_28_29_full_fidelity_kfx_export_and_roundtrip() {
    let container_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    let opf_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="uid">urn:uuid:12345678-1234-5678-1234-567812345678</dc:identifier>
    <dc:title>Full KFX Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:language>ar</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
    <item id="img1" href="images/dot.png" media-type="image/png"/>
  </manifest>
  <spine page-progression-direction="rtl">
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#;

    let nav_xhtml = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>TOC</title></head>
<body>
  <nav epub:type="toc">
    <ol>
      <li><a href="chapter1.xhtml">Chapter One</a>
        <ol>
          <li><a href="chapter1.xhtml#sub1">Subsection 1.1</a></li>
        </ol>
      </li>
      <li><a href="chapter2.xhtml#detail">Chapter Two</a></li>
    </ol>
  </nav>
</body>
</html>"#;

    let ch1_xhtml = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body>
  <h1>Chapter One</h1>
  <p id="sub1">Visit <a href="https://example.org/kfx-check">Example Site</a> or jump to <a href="chapter2.xhtml#detail">Details</a>.</p>
  <p><img src="images/dot.png" alt="Tiny Dot"/></p>
  <table>
    <tr><th colspan="2">Header</th></tr>
    <tr><td>Cell A1</td><td>Cell B1</td></tr>
  </table>
</body>
</html>"#;

    let ch2_xhtml = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 2</title></head>
<body>
  <p id="detail">Target detail paragraph in chapter two.</p>
</body>
</html>"#;

    let epub_bytes = build_test_epub(&[
        ("META-INF/container.xml", container_xml),
        ("OEBPS/content.opf", opf_xml),
        ("OEBPS/nav.xhtml", nav_xhtml),
        ("OEBPS/chapter1.xhtml", ch1_xhtml),
        ("OEBPS/chapter2.xhtml", ch2_xhtml),
        ("OEBPS/images/dot.png", TINY_PNG),
    ]);

    let book = Book::from_bytes(&epub_bytes).expect("EPUB should parse");
    assert_eq!(book.metadata().direction, PageProgressionDirection::Rtl);

    let kfx_bytes = UniversalKfxExporter::export(&book).expect("KFX export should succeed");
    let c = KfxContainer::parse(&kfx_bytes).unwrap();

    // Verify RTL direction in $538 document_data (#28)
    let doc_data = c
        .index_entries
        .iter()
        .find(|e| e.type_id == SYM_DOCUMENT_DATA)
        .map(|e| decode_enty(c.get_entry_payload(e).unwrap()))
        .unwrap();
    assert_eq!(
        doc_data.get_field(192).and_then(|v| v.as_symbol()),
        Some(375)
    );

    // Verify embedded image ($164 + $417) (#25)
    assert!(c.index_entries.iter().any(|e| e.type_id == 164));
    assert!(c.index_entries.iter().any(|e| e.type_id == 417));

    // Verify anchors ($266) for external + internal links (#27)
    let anchors: Vec<IonValue> = c
        .index_entries
        .iter()
        .filter(|e| e.type_id == 266)
        .map(|e| decode_enty(c.get_entry_payload(e).unwrap()))
        .collect();
    assert!(anchors.iter().any(
        |a| a.get_field(186).and_then(|v| v.as_str()) == Some("https://example.org/kfx-check")
    ));
    assert!(anchors.iter().any(|a| a.get_field(183).is_some()));

    // Verify round-trip via KfxBook::parse
    let parsed = KfxBook::parse(&kfx_bytes).unwrap();
    assert_eq!(parsed.metadata().direction, PageProgressionDirection::Rtl);
    assert_eq!(parsed.toc().len(), 2);
    assert_eq!(parsed.toc()[0].label, "Chapter One");
    assert_eq!(parsed.toc()[0].subitems.len(), 1);
    assert_eq!(parsed.toc()[0].subitems[0].label, "Subsection 1.1");
    assert_eq!(parsed.toc()[1].label, "Chapter Two");

    let ch1_out = parsed.get_section(0).unwrap();
    assert!(ch1_out.raw_html.contains("<table"));
    assert!(ch1_out.raw_html.contains("colspan=\"2\""));
    assert!(ch1_out.raw_html.contains("Cell A1"));
    assert!(ch1_out.raw_html.contains("<img src=\"images/"));
    assert!(ch1_out.raw_html.contains("https://example.org/kfx-check"));
    assert!(ch1_out.raw_html.contains("sec_1.xhtml#eid_"));
    assert!(!parsed.archive().list_files().is_empty());
}

#[test]
fn kfx_content_fragment_chunking_under_8192_limit() {
    let mut big_md = String::from("# Chunk Test\n\n");
    for i in 0..40 {
        big_md.push_str(&format!("Paragraph {i}: {}\n\n", "word ".repeat(80)));
    }
    let book = TxtBook::parse(big_md.as_bytes(), "Chunk Book", true).unwrap();
    let kfx_bytes = UniversalKfxExporter::export(&book).unwrap();
    let c = KfxContainer::parse(&kfx_bytes).unwrap();

    let content_entries: Vec<_> = c
        .index_entries
        .iter()
        .filter(|e| e.type_id == SYM_CONTENT_BODY)
        .collect();
    assert!(
        content_entries.len() > 1,
        "Expected multiple $145 content chunks"
    );

    for entry in content_entries {
        let val = decode_enty(c.get_entry_payload(entry).unwrap());
        let list = val.get_field(146).unwrap().as_list().unwrap();
        let bytes_before_last: usize = list[..list.len().saturating_sub(1)]
            .iter()
            .map(|v| v.as_str().unwrap_or("").len())
            .sum();
        assert!(
            bytes_before_last < 8192,
            "Content chunk exceeded 8192 limit: {bytes_before_last}"
        );
    }
}
