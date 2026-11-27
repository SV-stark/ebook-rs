use ebook_rs::opds::OpdsFeed;

#[test]
fn test_blackbox_opds_atom_xml_feed_parsing() {
    let atom_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>urn:uuid:opds-feed-1</id>
  <title>Sample OPDS Catalog</title>
  <entry>
    <id>urn:uuid:book-1</id>
    <title>OPDS Book One</title>
    <author><name>Jane Doe</name></author>
    <link rel="http://opds-spec.org/acquisition" href="http://example.com/book1.epub" type="application/epub+zip"/>
  </entry>
</feed>"#;

    let feed = OpdsFeed::parse_atom_xml(atom_xml).expect("OPDS ATOM feed should parse");
    assert_eq!(feed.title, "Sample OPDS Catalog");
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].title, "OPDS Book One");
    assert!(feed.entries[0].authors.iter().any(|a| a == "Jane Doe"));
    let acq = feed.entries[0].download_link(Some("application/epub+zip"));
    assert!(acq.is_some());
    assert_eq!(acq.unwrap().href, "http://example.com/book1.epub");
}

#[test]
fn test_blackbox_opds_json_2_feed_parsing() {
    let opds_json = r#"{
        "metadata": {
            "title": "OPDS 2.0 Catalog"
        },
        "publications": [
            {
                "metadata": {
                    "title": "OPDS JSON Book"
                },
                "links": [
                    { "rel": "http://opds-spec.org/acquisition", "href": "http://example.com/book2.epub", "type": "application/epub+zip" }
                ]
            }
        ]
    }"#;

    let feed = OpdsFeed::parse_json(opds_json).expect("OPDS JSON feed should parse");
    assert_eq!(feed.title, "OPDS 2.0 Catalog");
    assert_eq!(feed.entries.len(), 1);
    assert_eq!(feed.entries[0].title, "OPDS JSON Book");
}

#[test]
fn test_opds_2_extended_fields() {
    let opds_json = r#"{
        "metadata": {
            "title": "Extended OPDS 2.0",
            "modified": "2026-11-27T12:00:00Z"
        },
        "navigation": [
            { "href": "/new", "type": "application/opds+json", "title": "New Releases" }
        ],
        "publications": [
            {
                "metadata": {
                    "title": "Book A",
                    "author": "Alice Author",
                    "modified": "2026-11-26T00:00:00Z"
                },
                "links": [
                    { "href": "/a.epub", "type": "application/epub+zip" }
                ],
                "images": [
                    { "href": "/a.jpg", "type": "image/jpeg" }
                ]
            }
        ],
        "groups": [
            {
                "publications": [
                    {
                        "metadata": {
                            "title": "Book B",
                            "author": ["Bob Author", { "name": "Carol Author" }]
                        },
                        "links": [
                            { "href": "/b.epub", "type": "application/epub+zip" }
                        ]
                    }
                ]
            }
        ]
    }"#;

    let feed = OpdsFeed::parse_json(opds_json).expect("Extended OPDS 2.0 JSON should parse");
    assert_eq!(feed.updated.as_deref(), Some("2026-11-27T12:00:00Z"));
    assert_eq!(feed.links.len(), 1);
    assert_eq!(feed.links[0].rel, "subsection");
    assert_eq!(feed.entries.len(), 2);
    assert_eq!(feed.entries[0].authors, vec!["Alice Author"]);
    assert_eq!(
        feed.entries[0].updated.as_deref(),
        Some("2026-11-26T00:00:00Z")
    );
    assert!(
        feed.entries[0]
            .links
            .iter()
            .any(|l| l.rel == "http://opds-spec.org/image" && l.href == "/a.jpg")
    );
    assert_eq!(feed.entries[1].authors, vec!["Bob Author", "Carol Author"]);
}
