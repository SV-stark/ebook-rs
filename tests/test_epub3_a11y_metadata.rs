// EPUB 3 Accessibility Metadata test (v0.5.3)
use ebook_rs::opf::parse_opf;

#[test]
fn test_epub3_accessibility_metadata_parsing() {
    let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="pub-id">
        <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
            <dc:title>Accessible EPUB 3 Book</dc:title>
            <dc:language>en</dc:language>
            <meta property="schema:accessMode">textual</meta>
            <meta property="schema:accessMode">visual</meta>
            <meta property="schema:accessModeSufficient">textual</meta>
            <meta property="schema:accessibilityFeature">alternativeText</meta>
            <meta property="schema:accessibilityFeature">structuralNavigation</meta>
            <meta property="schema:accessibilityHazard">none</meta>
            <meta property="schema:accessibilitySummary">This book contains detailed alternative text for all images and full structural navigation.</meta>
            <meta property="a11y:certifiedBy">DAISY Consortium</meta>
        </metadata>
        <manifest>
            <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
        </manifest>
        <spine>
            <itemref idref="ch1"/>
        </spine>
    </package>"#;

    let opf = parse_opf(opf_xml, "content.opf").expect("Should parse OPF");
    let a11y = &opf.metadata.accessibility;

    assert!(a11y.is_accessible);
    assert_eq!(a11y.access_modes, vec!["textual", "visual"]);
    assert_eq!(
        a11y.accessibility_features,
        vec!["alternativeText", "structuralNavigation"]
    );
    assert_eq!(a11y.accessibility_hazards, vec!["none"]);
    assert_eq!(
        a11y.accessibility_summary.as_deref(),
        Some(
            "This book contains detailed alternative text for all images and full structural navigation."
        )
    );
    assert_eq!(a11y.certified_by.as_deref(), Some("DAISY Consortium"));

    assert!(a11y.has_alternative_text());
    assert!(a11y.has_structural_navigation());
    assert!(a11y.is_screen_reader_friendly());
}
