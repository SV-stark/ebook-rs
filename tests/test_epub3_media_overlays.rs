use ebook_rs::media_overlay::{MediaOverlayPackage, SmilClock};

#[test]
fn test_smil_clock_parsing() {
    assert_eq!(SmilClock::parse_npt_seconds("00:01:23.45"), 83.45);
    assert_eq!(SmilClock::parse_npt_seconds("83.45s"), 83.45);
    assert_eq!(SmilClock::parse_npt_seconds("1:23.45"), 83.45);
    assert_eq!(SmilClock::format_npt(83.45), "00:01:23.450");
}

#[test]
fn test_epub3_smil_media_overlay_parsing() {
    let smil_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <smil xmlns="http://www.w3.org/ns/SMIL" xmlns:epub="http://www.idpf.org/2007/ops" version="3.0">
        <body>
            <seq id="seq1" epub:textref="chapter1.xhtml">
                <par id="par1">
                    <text src="chapter1.xhtml#p1"/>
                    <audio src="audio/ch1.mp3" clipBegin="00:00:05.120" clipEnd="00:00:12.450"/>
                </par>
                <par id="par2">
                    <text src="chapter1.xhtml#p2"/>
                    <audio src="audio/ch1.mp3" clipBegin="00:00:12.450" clipEnd="00:00:20.100"/>
                </par>
            </seq>
        </body>
    </smil>"#;

    let pkg = MediaOverlayPackage::parse_smil(smil_xml, "OEBPS/overlay/ch1.smil")
        .expect("Should parse SMIL XML");

    assert_eq!(pkg.sequences.len(), 1);
    let seq = &pkg.sequences[0];
    assert_eq!(seq.id.as_deref(), Some("seq1"));
    assert_eq!(seq.epub_textref.as_deref(), Some("chapter1.xhtml"));
    assert_eq!(seq.parallels.len(), 2);

    let par1 = &seq.parallels[0];
    let text1 = par1.text.as_ref().unwrap();
    let audio1 = par1.audio.as_ref().unwrap();

    assert_eq!(text1.src, "chapter1.xhtml#p1");
    assert_eq!(text1.element_id.as_deref(), Some("p1"));
    assert_eq!(audio1.src, "audio/ch1.mp3");
    assert_eq!(audio1.clip_begin, 5.12);
    assert_eq!(audio1.clip_end, 12.45);

    // Test reverse timestamp lookup
    let matched_text = pkg.find_text_ref_by_timestamp("ch1.mp3", 8.0).unwrap();
    assert_eq!(matched_text.element_id.as_deref(), Some("p1"));

    let matched_text2 = pkg.find_text_ref_by_timestamp("ch1.mp3", 15.0).unwrap();
    assert_eq!(matched_text2.element_id.as_deref(), Some("p2"));

    // Test text href lookup
    let matched_audio = pkg.find_audio_clip_by_text_href("chapter1.xhtml#p2").unwrap();
    assert_eq!(matched_audio.clip_begin, 12.45);
    assert_eq!(matched_audio.clip_end, 20.1);

    // Test JSON export
    let json = pkg.to_smil_json().unwrap();
    assert!(json.contains("audio/ch1.mp3"));
    assert!(json.contains("chapter1.xhtml#p1"));
}
