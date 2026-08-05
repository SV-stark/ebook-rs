use ebook_rs::nav::{parse_nav_xhtml, parse_ncx};
use ebook_rs::opf::{parse_container_xml, parse_opf};

#[test]
fn test_parse_container_xml() {
    let xml = r#"<?xml version="1.0"?>
    <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
        <rootfiles>
            <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
        </rootfiles>
    </container>"#;

    let path = parse_container_xml(xml).unwrap();
    assert_eq!(path, "OEBPS/content.opf");
}

#[test]
fn test_parse_opf_metadata_manifest_spine() {
    let xml = r#"<?xml version="1.0"?>
    <package xmlns="http://www.idpf.org/2007/opf" version="3.0">
        <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
            <dc:title>Test Book Title</dc:title>
            <dc:creator>Test Author</dc:creator>
            <dc:language>en</dc:language>
        </metadata>
        <manifest>
            <item id="ch1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/>
        </manifest>
        <spine>
            <itemref idref="ch1"/>
        </spine>
    </package>"#;

    let opf = parse_opf(xml, "OEBPS/content.opf").unwrap();
    assert_eq!(opf.metadata.title, "Test Book Title");
    assert_eq!(opf.metadata.creators, vec!["Test Author"]);
    assert_eq!(opf.spine.len(), 1);
    assert_eq!(opf.spine[0].href, "OEBPS/text/ch1.xhtml");
}

#[test]
fn test_parse_ncx_toc() {
    let ncx = r#"<?xml version="1.0"?>
    <ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">
        <navMap>
            <navPoint id="np1">
                <navLabel><text>Chapter One</text></navLabel>
                <content src="ch1.xhtml"/>
            </navPoint>
        </navMap>
    </ncx>"#;

    let toc = parse_ncx(ncx, "OEBPS/toc.ncx").unwrap();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].label, "Chapter One");
    assert_eq!(toc[0].full_path, "OEBPS/ch1.xhtml");
}

#[test]
fn test_parse_nav_xhtml_toc() {
    let nav = r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
    <body>
        <nav epub:type="toc">
            <ol>
                <li><a href="ch1.xhtml">Chapter One</a></li>
            </ol>
        </nav>
    </body>
    </html>"#;

    let toc = parse_nav_xhtml(nav, "OEBPS/nav.xhtml").unwrap();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].label, "Chapter One");
    assert_eq!(toc[0].full_path, "OEBPS/ch1.xhtml");
}
