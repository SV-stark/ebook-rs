use ebook_rs::{decompress_palmdoc, Book, MobiBook};

#[test]
fn test_palmdoc_lz77_decompression() {
    // Literal bytes test
    let data = b"\x05Hello world!";
    let decompressed = decompress_palmdoc(data);
    assert!(String::from_utf8_lossy(&decompressed).contains("Hello"));

    // Space + byte test (0xC0 + 'A'^0x80)
    let space_byte = vec![0xC0 | b'A' ^ 0x80];
    let decomp_space = decompress_palmdoc(&space_byte);
    assert_eq!(decomp_space, vec![b' ', b'A']);
}

#[test]
fn test_mobi_pdb_header_parsing_and_builder() {
    let mut pdb_bytes = vec![0u8; 78];

    // PDB name: "Test Mobi Book"
    let name = b"Test Mobi Book";
    pdb_bytes[..name.len()].copy_from_slice(name);

    // PDB record count = 2
    pdb_bytes[76] = 0;
    pdb_bytes[77] = 2;

    // Record offsets
    let rec0_start = 78 + 2 * 8; // 94
    let rec0_len = 56;
    let rec1_start = rec0_start + rec0_len; // 150

    pdb_bytes.extend_from_slice(&(rec0_start as u32).to_be_bytes());
    pdb_bytes.extend_from_slice(&0u32.to_be_bytes());
    pdb_bytes.extend_from_slice(&(rec1_start as u32).to_be_bytes());
    pdb_bytes.extend_from_slice(&0u32.to_be_bytes());

    // Record 0: PalmDOC header (16 bytes) + MOBI header (40 bytes)
    let mut rec0 = vec![0u8; rec0_len];
    rec0[0] = 0;
    rec0[1] = 1; // compression = 1 (uncompressed)
    rec0[8] = 0;
    rec0[9] = 1; // text record count = 1
    rec0[16..20].copy_from_slice(b"MOBI");
    rec0[20..24].copy_from_slice(&24u32.to_be_bytes());

    pdb_bytes.extend_from_slice(&rec0);

    // Record 1: HTML Content
    let html_content = b"<html><body><h1>Chapter 1</h1><p>MOBI text content</p></body></html>";
    pdb_bytes.extend_from_slice(html_content);

    // Parse MOBI binary into Book struct
    let mobi = MobiBook::parse(&pdb_bytes).expect("Should parse MOBI PDB");
    assert_eq!(mobi.metadata().title, "Test Mobi Book");
    assert_eq!(mobi.spine().len(), 1);

    let sec0 = mobi.get_section(0).expect("Section 0");
    assert!(sec0.raw_html.contains("MOBI text content"));
    assert!(sec0.plain_text.contains("Chapter 1 MOBI text content"));

    // Test unified Book::from_bytes auto-detection
    let book = Book::from_bytes(&pdb_bytes).expect("Auto-detect MOBI");
    assert_eq!(book.metadata().title, "Test Mobi Book");
}
