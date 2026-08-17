use ebook_rs::media_overlay::MediaOverlayPackage;
use ebook_rs::optimizer::{EpubOptimizer, EpubOptimizerOptions};
use ebook_rs::paginator::{ReflowPaginator, WritingMode};
use ebook_rs::{TxtBook, UniBook};

#[test]
fn test_vertical_cjk_and_rtl_paginator() {
    let english_text = "The quick brown fox jumps over the lazy dog repeatedly to fill up paragraph space for pagination testing.";
    let japanese_text = "吾輩は猫である。名前はまだ無い。どこで生れたかとんと見当がつかぬ。何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。";
    let arabic_text = "كان ياما كان في قديم الزمان وسالف العصر والأوان، قرية هادئة على ضفاف النهر.";

    // 1. Vertical RL pagination for Japanese CJK
    let vert_paginator =
        ReflowPaginator::new(16, 1.6, 400, 600, 20).with_writing_mode(WritingMode::VerticalRl);
    assert!(vert_paginator.is_vertical());
    assert!(vert_paginator.is_rtl());
    assert!(
        vert_paginator
            .css_properties()
            .contains("writing-mode: vertical-rl")
    );

    let jp_map = vert_paginator.paginate_text(japanese_text);
    assert!(jp_map.total_pages >= 1);
    assert_eq!(jp_map.writing_mode, WritingMode::VerticalRl);
    assert!(jp_map.is_rtl);

    // 2. Horizontal RTL for Arabic
    let rtl_paginator =
        ReflowPaginator::new(16, 1.6, 600, 400, 20).with_writing_mode(WritingMode::HorizontalRtl);
    assert!(!rtl_paginator.is_vertical());
    assert!(rtl_paginator.is_rtl());
    assert!(rtl_paginator.css_properties().contains("direction: rtl"));

    let ar_map = rtl_paginator.paginate_text(arabic_text);
    assert!(ar_map.total_pages >= 1);
    assert_eq!(ar_map.writing_mode, WritingMode::HorizontalRtl);

    // 3. Standard LTR
    let ltr_paginator = ReflowPaginator::default();
    assert!(!ltr_paginator.is_vertical());
    assert!(!ltr_paginator.is_rtl());
    let en_map = ltr_paginator.paginate_text(english_text);
    assert!(en_map.total_pages >= 1);
}

#[test]
fn test_media_overlay_web_audio_and_karaoke_highlighting() {
    let smil_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<smil xmlns="http://www.w3.org/ns/SMIL" xmlns:epub="http://www.idpf.org/2007/ops" version="3.0">
  <body>
    <seq id="seq1" epub:textref="chapter1.xhtml">
      <par id="par1">
        <text src="chapter1.xhtml#p1"/>
        <audio src="audio/ch1.mp3" clipBegin="00:00:01.500" clipEnd="00:00:05.200"/>
      </par>
      <par id="par2">
        <text src="chapter1.xhtml#p2"/>
        <audio src="audio/ch1.mp3" clipBegin="00:00:05.200" clipEnd="00:00:12.800"/>
      </par>
    </seq>
  </body>
</smil>"#;

    let package = MediaOverlayPackage::parse_smil(smil_xml, "OEBPS/media/ch1.smil").unwrap();
    let cue_sheet = package.to_karaoke_cue_sheet();
    assert_eq!(cue_sheet.cues.len(), 2);
    assert_eq!(cue_sheet.cues[0].element_id.as_deref(), Some("p1"));
    assert!((cue_sheet.cues[0].clip_begin - 1.5).abs() < 1e-4);
    assert!((cue_sheet.cues[0].clip_end - 5.2).abs() < 1e-4);
    assert!((cue_sheet.total_duration - 12.8).abs() < 1e-4);

    let html_content =
        "<div class=\"chapter\"><p id=\"p1\">First line</p><p id=\"p2\">Second line</p></div>";
    let annotated = package.annotate_html_with_media_overlays(html_content);
    assert!(annotated.contains("data-audio-src="));
    assert!(annotated.contains("data-clip-begin=\"1.500\""));
    assert!(annotated.contains("data-clip-end=\"5.200\""));
    assert!(annotated.contains("media-overlay-active-target"));

    let manifest_json = package.generate_web_audio_manifest().unwrap();
    assert!(manifest_json.contains("total_duration"));
}

#[test]
fn test_markdown_frontmatter_wikilinks_and_callouts() {
    let md_content = r#"---
title: "The Rust Odyssey"
author: "Stark Developer"
language: "en"
tags: ["rust", "ebook", "parser"]
description: "A comprehensive guide to high performance parsers"
---

# Chapter One

Welcome to [[Chapter 2|Next Chapter]] or check the [[#glossary]].

> [!NOTE] Implementation Note
> This parser executes in pure Rust with zero external C dependencies.
> Make sure to utilize SIMD operations when possible.

> [!TIP]
> Memory mapping large files ensures instant startup.

Here is **bold text**, *italic emphasis*, and `inline code`.
"#;

    let book = TxtBook::parse(md_content.as_bytes(), "Default Title", true).unwrap();
    let meta = book.metadata();
    assert_eq!(meta.title, "The Rust Odyssey");
    assert_eq!(meta.creator(), "Stark Developer");
    assert_eq!(meta.language(), "en");
    assert_eq!(
        meta.description.as_deref(),
        Some("A comprehensive guide to high performance parsers")
    );

    let sec_html = &book.get_section(0).unwrap().processed_html;
    // Verify wikilinks rendered as <a> tags
    assert!(
        sec_html.contains("<a href=\"#ch-next-chapter\" class=\"wikilink\">Next Chapter</a>")
            || sec_html.contains("class=\"wikilink\"")
    );
    assert!(sec_html.contains("<a href=\"#glossary\" class=\"wikilink\">glossary</a>"));

    // Verify callouts rendered properly
    assert!(sec_html.contains("class=\"callout callout-note\""));
    assert!(sec_html.contains("Implementation Note"));
    assert!(sec_html.contains("class=\"callout callout-tip\""));

    // Verify inline formatting
    assert!(sec_html.contains("<strong>bold text</strong>"));
    assert!(sec_html.contains("<em>italic emphasis</em>"));
    assert!(sec_html.contains("<code>inline code</code>"));
}

#[test]
fn test_epub3_optimizer_and_minifier() {
    let raw_html = "<!-- This is a comment --><div   class=\"content main-text\"  id=\"intro\"  >  <p>  Hello   World!  </p>  </div>";
    let minified_html = EpubOptimizer::minify_html(raw_html);
    assert!(!minified_html.contains("This is a comment"));
    assert!(!minified_html.contains("   "));

    let raw_css = "/* Main stylesheet */ \n .content { color: red ; margin: 10px ; } \n .unused-class { display: none; } \n #intro { font-weight: bold; }";
    let minified_css = EpubOptimizer::minify_css(raw_css);
    assert!(!minified_css.contains("/* Main stylesheet */"));

    let mut used_classes = ahash::AHashSet::new();
    used_classes.insert("content".to_string());
    let mut used_ids = ahash::AHashSet::new();
    used_ids.insert("intro".to_string());
    let used_tags = ahash::AHashSet::new();

    let (purged_css, count) =
        EpubOptimizer::purge_css(raw_css, &used_classes, &used_ids, &used_tags);
    assert_eq!(count, 1);
    assert!(!purged_css.contains(".unused-class"));
    assert!(purged_css.contains(".content"));
    assert!(purged_css.contains("#intro"));

    // Test book export optimization
    let md = "# Test Book\n\nSome book paragraph.";
    let book = TxtBook::parse(md.as_bytes(), "Test Book", true).unwrap();
    let opt_options = EpubOptimizerOptions::default();
    let opt_bytes = book.export_optimized_epub3_bytes(&opt_options).unwrap();
    assert!(!opt_bytes.is_empty());
}

#[test]
fn test_uniffi_unibook_wrapper() {
    let md = "# UniFFI Test Book\n\nFirst paragraph for mobile reader verification.";
    let book = TxtBook::parse(md.as_bytes(), "UniFFI Test Book", true).unwrap();
    let epub_bytes = book.export_epub3_bytes().unwrap();

    let unibook = UniBook::from_bytes(epub_bytes).unwrap();
    assert_eq!(unibook.get_title(), "UniFFI Test Book");
    assert!(unibook.get_sections_count() >= 1);

    let sec_html = unibook.get_section_html(0).unwrap();
    assert!(sec_html.contains("First paragraph for mobile reader"));

    let search_results = unibook.search("mobile".to_string(), false);
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].spine_index, 0);

    let paginated_json = unibook.paginate_section(0, 16, 400, 600, false).unwrap();
    assert!(paginated_json.contains("total_pages"));

    let opt_epub = unibook.export_optimized_epub3().unwrap();
    assert!(!opt_epub.is_empty());
}
