use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Creates a valid sample EPUB 3 file with TOC, OPF, NCX, NAV, and chapters for testing.
pub fn generate_sample_epub() -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let cursor = Cursor::new(&mut buf);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    // 1. mimetype (must be uncompressed)
    let stored_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", stored_options)
        .map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| e.to_string())?;

    // 2. META-INF/container.xml
    zip.start_file("META-INF/container.xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
    <rootfiles>
        <rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/>
    </rootfiles>
</container>"#,
    )
    .map_err(|e| e.to_string())?;

    // 3. OEBPS/package.opf
    zip.start_file("OEBPS/package.opf", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id">
    <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
        <dc:identifier id="pub-id">urn:uuid:12345678-abcd-efgh-1234-56789abcdef0</dc:identifier>
        <dc:title>The Rustonomicon &amp; EBook-RS Guide</dc:title>
        <dc:creator>Antigravity AI</dc:creator>
        <dc:language>en</dc:language>
        <dc:publisher>Rust Ebook Publishers</dc:publisher>
        <dc:description>A complete sample EPUB 3 book for testing the ebook-rs parser and reader.</dc:description>
        <meta property="dcterms:modified">2026-08-05T20:30:00Z</meta>
    </metadata>
    <manifest>
        <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
        <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
        <item id="style" href="style.css" media-type="text/css"/>
        <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
        <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
        <item id="ch3" href="ch3.xhtml" media-type="application/xhtml+xml"/>
    </manifest>
    <spine toc="ncx">
        <itemref idref="ch1"/>
        <itemref idref="ch2"/>
        <itemref idref="ch3"/>
    </spine>
</package>"#).map_err(|e| e.to_string())?;

    // 4. OEBPS/toc.ncx
    zip.start_file("OEBPS/toc.ncx", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
    <head>
        <meta name="dtb:uid" content="urn:uuid:12345678-abcd-efgh-1234-56789abcdef0"/>
    </head>
    <docTitle><text>The Rustonomicon &amp; EBook-RS Guide</text></docTitle>
    <navMap>
        <navPoint id="np-1" playOrder="1">
            <navLabel><text>Chapter 1: Welcome to EBook-RS</text></navLabel>
            <content src="ch1.xhtml"/>
        </navPoint>
        <navPoint id="np-2" playOrder="2">
            <navLabel><text>Chapter 2: Canonical Fragment Identifiers (CFI)</text></navLabel>
            <content src="ch2.xhtml"/>
        </navPoint>
        <navPoint id="np-3" playOrder="3">
            <navLabel><text>Chapter 3: Full-Text Search and Annotations</text></navLabel>
            <content src="ch3.xhtml"/>
        </navPoint>
    </navMap>
</ncx>"#,
    )
    .map_err(|e| e.to_string())?;

    // 5. OEBPS/nav.xhtml
    zip.start_file("OEBPS/nav.xhtml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
    <title>Table of Contents</title>
    <link rel="stylesheet" href="style.css" type="text/css"/>
</head>
<body>
    <nav epub:type="toc" id="toc">
        <h1>Table of Contents</h1>
        <ol>
            <li><a href="ch1.xhtml">Chapter 1: Welcome to EBook-RS</a></li>
            <li><a href="ch2.xhtml">Chapter 2: Canonical Fragment Identifiers (CFI)</a></li>
            <li><a href="ch3.xhtml">Chapter 3: Full-Text Search and Annotations</a></li>
        </ol>
    </nav>
</body>
</html>"#,
    )
    .map_err(|e| e.to_string())?;

    // 6. OEBPS/style.css
    zip.start_file("OEBPS/style.css", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        br#"
        body { font-family: sans-serif; line-height: 1.6; padding: 20px; }
        h1 { color: #2563eb; }
        p { margin-bottom: 1rem; }
    "#,
    )
    .map_err(|e| e.to_string())?;

    // 7. OEBPS/ch1.xhtml
    zip.start_file("OEBPS/ch1.xhtml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title><link rel="stylesheet" href="style.css"/></head>
<body>
    <h1>Chapter 1: Welcome to EBook-RS</h1>
    <p>Welcome to <strong>ebook-rs</strong>, a high-performance pure Rust EPUB parser and reader library built with feature parity to <em>epub.js</em>.</p>
    <p>This library parses EPUB 2 and EPUB 3 archives with complete support for OPF package metadata, manifests, spine navigation, NCX, and XHTML navigation documents.</p>
</body>
</html>"#).map_err(|e| e.to_string())?;

    // 8. OEBPS/ch2.xhtml
    zip.start_file("OEBPS/ch2.xhtml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 2</title><link rel="stylesheet" href="style.css"/></head>
<body>
    <h1>Chapter 2: Canonical Fragment Identifiers (CFI)</h1>
    <p>EPUB CFI is an IDPF standard for referencing exact structural elements and character offsets inside EPUB documents.</p>
    <p>For example, <code>epubcfi(/6/4[chap01ref]!/4/2/10/1:5)</code> specifies spine item index 1, body tag step 4, div tag step 2, text node 1, offset 5.</p>
</body>
</html>"#).map_err(|e| e.to_string())?;

    // 9. OEBPS/ch3.xhtml
    zip.start_file("OEBPS/ch3.xhtml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 3</title><link rel="stylesheet" href="style.css"/></head>
<body>
    <h1>Chapter 3: Full-Text Search and Annotations</h1>
    <p>With ebook-rs, full-text searching scans all spine items asynchronously or synchronously, producing search matches complete with surrounding context snippets and exact CFIs.</p>
    <p>Annotations like highlights, bookmarks, and text notes are mapped to CFIs and saved as structured JSON.</p>
</body>
</html>"#).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(buf)
}
