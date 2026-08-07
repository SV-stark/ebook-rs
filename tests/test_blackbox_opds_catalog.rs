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
