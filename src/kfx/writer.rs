use crate::book::Book;
use crate::kfx::container::{KFX_HEADER_LEN, KFX_MAGIC_CONT, KfxIndexEntry};
use crate::kfx::ion::{IonValue, encode_stream};
use crate::kfx::symbols::*;
use sha1_smol::Sha1;

const YJ_MAX_ID: u32 = 851;
const SYM_NULL_EID: u32 = 348;
const ENTY_PREAMBLE: [u8; 21] =
    *b"ENTY\x01\x00\x15\x00\x00\x00\xe0\x01\x00\xea\xd6\x03\x9a\x20\x03\x9b\x20";

/// Universal Amazon KFX Container Exporter capable of serializing any `Book` into valid KFX container bytes.
pub struct UniversalKfxExporter;

impl UniversalKfxExporter {
    /// Export any `Book` (EPUB, MOBI, AZW3, PDF, FB2, TXT, MD) to a valid Amazon KFX binary container buffer.
    pub fn export(book: &Book) -> Result<Vec<u8>, String> {
        let meta = book.metadata();
        let title = if meta.title.trim().is_empty() {
            "Untitled"
        } else {
            meta.title.trim()
        };
        let author = meta
            .creators
            .first()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("Unknown Author");
        let lang = if meta.language().trim().is_empty() {
            "en"
        } else {
            meta.language().trim()
        };

        let container_id = "CR!EBOOKRS000000000000000000001";
        let asin = "B00EBOOKRS0000000000000000000001";
        let book_id = "ebookrs_book_0000000001";

        // Collect non-empty chapter paragraph lists
        let mut chapters: Vec<Vec<String>> = Vec::new();
        for idx in 0..book.spine().len() {
            if let Ok(sec) = book.get_section(idx) {
                let text_src = if !sec.plain_text.trim().is_empty() {
                    sec.plain_text.trim()
                } else if !sec.raw_html.trim().is_empty() {
                    sec.raw_html.trim()
                } else {
                    ""
                };
                if !text_src.is_empty() {
                    let paras: Vec<String> = text_src
                        .split('\n')
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .map(ToOwned::to_owned)
                        .collect();
                    if !paras.is_empty() {
                        chapters.push(paras);
                    }
                }
            }
        }
        if chapters.is_empty() {
            chapters.push(vec![title.to_string()]);
        }

        // Build local symbol table starting at YJ_MAX_ID + 1 (852)
        // 852 = "s0" (shared paragraph style)
        // For each chapter i: "c{i}", "l{i}", "content_{i}"
        let style_sym = YJ_MAX_ID + 1;
        let mut local_symbols = vec!["s0".to_string()];
        let mut chap_syms = Vec::with_capacity(chapters.len());
        for i in 0..chapters.len() {
            let base = YJ_MAX_ID + 1 + local_symbols.len() as u32;
            local_symbols.push(format!("c{i}"));
            local_symbols.push(format!("l{i}"));
            local_symbols.push(format!("content_{i}"));
            chap_syms.push((base, base + 1, base + 2));
        }
        let max_sym_id = YJ_MAX_ID + local_symbols.len() as u32;

        let mut entries: Vec<KfxIndexEntry> = Vec::new();
        let mut payload: Vec<u8> = Vec::new();

        let mut push_entity = |entity_id: u32, type_id: u32, val: IonValue| {
            let ion_bytes = encode_stream(&[val]);
            let offset = payload.len() as u64;
            let length = (ENTY_PREAMBLE.len() + ion_bytes.len()) as u64;
            payload.extend_from_slice(&ENTY_PREAMBLE);
            payload.extend_from_slice(&ion_bytes);
            entries.push(KfxIndexEntry {
                entity_id,
                type_id,
                offset,
                length,
            });
        };

        // 1. $585 content_features
        let make_feature = |ns: &str, key: &str, maj: i64| {
            IonValue::Struct(vec![
                (586, IonValue::String(ns.to_string())),
                (492, IonValue::String(key.to_string())),
                (
                    589,
                    IonValue::Struct(vec![(
                        5,
                        IonValue::Struct(vec![(587, IonValue::Int(maj)), (588, IonValue::Int(0))]),
                    )]),
                ),
            ])
        };
        push_entity(
            SYM_NULL_EID,
            SYM_CONTENT_FEATURES,
            IonValue::Struct(vec![(
                590,
                IonValue::List(vec![
                    make_feature("com.amazon.yjconversion", "reflow-style", 1),
                    make_feature("SDK.Marker", "CanonicalFormat", 1),
                ]),
            )]),
        );

        // 2. $490 book_metadata
        let kv_str = |k: &str, v: &str| {
            IonValue::Struct(vec![
                (492, IonValue::String(k.to_string())),
                (307, IonValue::String(v.to_string())),
            ])
        };
        let kv_bool = |k: &str, v: bool| {
            IonValue::Struct(vec![
                (492, IonValue::String(k.to_string())),
                (307, IonValue::Bool(v)),
            ])
        };
        let mut title_meta = vec![
            kv_str("ASIN", asin),
            kv_str("asset_id", container_id),
            kv_str("author", author),
            kv_str("book_id", book_id),
            kv_str("cde_content_type", "PDOC"),
            kv_str("content_id", asin),
            kv_bool("is_sample", false),
            kv_str("language", lang),
            kv_bool("override_kindle_font", false),
            kv_str("title", title),
        ];
        if let Some(pub_name) = meta
            .publishers
            .first()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            title_meta.push(kv_str("publisher", pub_name));
        }
        if let Some(desc) = meta
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            title_meta.push(kv_str("description", desc));
        }

        push_entity(
            SYM_NULL_EID,
            SYM_BOOK_METADATA,
            IonValue::Struct(vec![(
                491,
                IonValue::List(vec![
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_audit_metadata".to_string())),
                        (
                            258,
                            IonValue::List(vec![
                                kv_str("file_creator", "KPR"),
                                kv_str("creator_version", "3.80.0"),
                            ]),
                        ),
                    ]),
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_title_metadata".to_string())),
                        (258, IonValue::List(title_meta)),
                    ]),
                    IonValue::Struct(vec![
                        (495, IonValue::String("kindle_ebook_metadata".to_string())),
                        (
                            258,
                            IonValue::List(vec![
                                kv_str("selection", "enabled"),
                                kv_str("nested_span", "enabled"),
                            ]),
                        ),
                    ]),
                    IonValue::Struct(vec![
                        (
                            495,
                            IonValue::String("kindle_capability_metadata".to_string()),
                        ),
                        (258, IonValue::List(Vec::new())),
                    ]),
                ]),
            )]),
        );

        let sec_sym_list: Vec<IonValue> = chap_syms
            .iter()
            .map(|(sec_s, _, _)| IonValue::Symbol(u64::from(*sec_s)))
            .collect();
        let reading_orders = IonValue::List(vec![IonValue::Struct(vec![
            (178, IonValue::Symbol(351)),
            (170, IonValue::List(sec_sym_list)),
        ])]);

        // 3. $258 metadata
        push_entity(
            SYM_NULL_EID,
            SYM_METADATA_RECORD,
            IonValue::Struct(vec![(169, reading_orders.clone())]),
        );

        // 4. $538 document_data
        push_entity(
            SYM_NULL_EID,
            SYM_DOCUMENT_DATA,
            IonValue::Struct(vec![
                (560, IonValue::Symbol(557)),
                (192, IonValue::Symbol(376)),
                (112, IonValue::Symbol(383)),
                (
                    16,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0x80, 0x01])),
                        (306, IonValue::Symbol(308)),
                    ]),
                ),
                (436, IonValue::Symbol(441)),
                (8, IonValue::Int(33)),
                (
                    42,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0xC1, 0x0C])),
                        (306, IonValue::Symbol(308)),
                    ]),
                ),
                (477, IonValue::Symbol(56)),
                (169, reading_orders),
            ]),
        );

        // 5. $389 book_navigation
        push_entity(
            SYM_NULL_EID,
            SYM_NAVIGATION_TOC,
            IonValue::List(vec![IonValue::Struct(vec![
                (178, IonValue::Symbol(351)),
                (392, IonValue::List(Vec::new())),
            ])]),
        );

        // 6. Sections ($260), Storylines ($259), Content ($145), and position tracking
        let mut next_eid: i64 = 1000;
        let mut sec_pos_map = Vec::new();
        let mut pos_id_map = Vec::new();
        let mut loc_items = Vec::new();
        let mut running_pid: i64 = 0;

        for (i, paras) in chapters.iter().enumerate() {
            let (sec_s, story_s, content_s) = chap_syms[i];
            let sec_page_eid = next_eid;
            next_eid += 1;

            let mut contains_ids = vec![IonValue::Int(sec_page_eid)];
            pos_id_map.push(IonValue::Struct(vec![
                (184, IonValue::Int(running_pid)),
                (185, IonValue::Int(sec_page_eid)),
            ]));
            running_pid += 1;

            let mut story_blocks = Vec::with_capacity(paras.len());
            let mut content_strings = Vec::with_capacity(paras.len());

            for (p_idx, para) in paras.iter().enumerate() {
                let blk_eid = next_eid;
                next_eid += 1;
                contains_ids.push(IonValue::Int(blk_eid));
                pos_id_map.push(IonValue::Struct(vec![
                    (184, IonValue::Int(running_pid)),
                    (185, IonValue::Int(blk_eid)),
                ]));
                loc_items.push(IonValue::Struct(vec![
                    (155, IonValue::Int(blk_eid)),
                    (143, IonValue::Int(0)),
                ]));
                running_pid += para.chars().count().max(1) as i64;

                story_blocks.push(IonValue::Struct(vec![
                    (155, IonValue::Int(blk_eid)),
                    (157, IonValue::Symbol(u64::from(style_sym))),
                    (159, IonValue::Symbol(269)),
                    (
                        145,
                        IonValue::Struct(vec![
                            (4, IonValue::Symbol(u64::from(content_s))),
                            (403, IonValue::Int(p_idx as i64)),
                        ]),
                    ),
                ]));
                content_strings.push(IonValue::String(para.clone()));
            }

            sec_pos_map.push(IonValue::Struct(vec![
                (181, IonValue::List(contains_ids)),
                (174, IonValue::Symbol(u64::from(sec_s))),
            ]));

            push_entity(
                sec_s,
                SYM_SECTION_BLOCK,
                IonValue::Struct(vec![
                    (174, IonValue::Symbol(u64::from(sec_s))),
                    (
                        141,
                        IonValue::List(vec![IonValue::Struct(vec![
                            (155, IonValue::Int(sec_page_eid)),
                            (176, IonValue::Symbol(u64::from(story_s))),
                            (159, IonValue::Symbol(269)),
                        ])]),
                    ),
                ]),
            );

            push_entity(
                story_s,
                SYM_STORYLINE_FRAGMENT,
                IonValue::Struct(vec![
                    (176, IonValue::Symbol(u64::from(story_s))),
                    (146, IonValue::List(story_blocks)),
                ]),
            );

            push_entity(
                content_s,
                SYM_CONTENT_BODY,
                IonValue::Struct(vec![
                    (4, IonValue::Symbol(u64::from(content_s))),
                    (146, IonValue::List(content_strings)),
                ]),
            );
        }

        pos_id_map.push(IonValue::Struct(vec![
            (184, IonValue::Int(running_pid)),
            (185, IonValue::Int(0)),
        ]));

        // 7. $157 style
        push_entity(
            style_sym,
            157,
            IonValue::Struct(vec![
                (
                    42,
                    IonValue::Struct(vec![
                        (307, IonValue::Decimal(vec![0x80, 0x01])),
                        (306, IonValue::Symbol(310)),
                    ]),
                ),
                (173, IonValue::Symbol(u64::from(style_sym))),
            ]),
        );

        // 8. $264 section_position_id_map, $265 position_id_map, $550 location_map, $395 resource_path
        push_entity(SYM_NULL_EID, 264, IonValue::List(sec_pos_map));
        push_entity(SYM_NULL_EID, 265, IonValue::List(pos_id_map));
        push_entity(
            SYM_NULL_EID,
            550,
            IonValue::List(vec![IonValue::Struct(vec![(
                182,
                IonValue::List(loc_items),
            )])]),
        );
        push_entity(
            SYM_NULL_EID,
            395,
            IonValue::Struct(vec![(247, IonValue::List(Vec::new()))]),
        );

        // 9. $419 container_entity_map
        let mut all_custom_eids = Vec::new();
        for (sec_s, story_s, content_s) in &chap_syms {
            all_custom_eids.push(IonValue::Symbol(u64::from(*sec_s)));
            all_custom_eids.push(IonValue::Symbol(u64::from(*story_s)));
            all_custom_eids.push(IonValue::Symbol(u64::from(*content_s)));
        }
        all_custom_eids.push(IonValue::Symbol(u64::from(style_sym)));

        push_entity(
            SYM_NULL_EID,
            419,
            IonValue::Struct(vec![(
                252,
                IonValue::List(vec![IonValue::Struct(vec![
                    (155, IonValue::String(container_id.to_string())),
                    (181, IonValue::List(all_custom_eids)),
                ])]),
            )]),
        );

        // Build header components: index_table, doc_symbol, format_capabilities, container_info, kfxgen_info
        let index_table_offset = KFX_HEADER_LEN;
        let index_table_length = entries.len() * 24;

        let doc_symbol_bytes = encode_stream(&[IonValue::Annotated(
            vec![3],
            Box::new(IonValue::Struct(vec![
                (8, IonValue::Int(i64::from(max_sym_id))),
                (
                    6,
                    IonValue::List(vec![IonValue::Struct(vec![
                        (4, IonValue::String("YJ_symbols".to_string())),
                        (5, IonValue::Int(10)),
                        (8, IonValue::Int(i64::from(YJ_MAX_ID))),
                    ])]),
                ),
                (
                    7,
                    IonValue::List(local_symbols.into_iter().map(IonValue::String).collect()),
                ),
            ])),
        )]);

        let fcap_bytes = encode_stream(&[IonValue::Annotated(
            vec![593],
            Box::new(IonValue::List(vec![IonValue::Struct(vec![
                (492, IonValue::String("kfxgen.textBlock".to_string())),
                (5, IonValue::Int(1)),
            ])])),
        )]);

        let doc_symbol_offset = index_table_offset + index_table_length;
        let doc_symbol_length = doc_symbol_bytes.len();
        let fcap_offset = doc_symbol_offset + doc_symbol_length;
        let fcap_length = fcap_bytes.len();
        let info_offset = fcap_offset + fcap_length;

        let info_bytes = encode_stream(&[IonValue::Struct(vec![
            (409, IonValue::String(container_id.to_string())),
            (410, IonValue::Int(0)),
            (411, IonValue::Int(0)),
            (
                SYM_INDEX_TABLE_OFFSET as u64,
                IonValue::Int(index_table_offset as i64),
            ),
            (
                SYM_INDEX_TABLE_LENGTH as u64,
                IonValue::Int(index_table_length as i64),
            ),
            (
                SYM_DOC_SYMBOL_OFFSET as u64,
                IonValue::Int(doc_symbol_offset as i64),
            ),
            (
                SYM_DOC_SYMBOL_LENGTH as u64,
                IonValue::Int(doc_symbol_length as i64),
            ),
            (412, IonValue::Int(4096)),
            (594, IonValue::Int(fcap_offset as i64)),
            (595, IonValue::Int(fcap_length as i64)),
        ])]);

        let mut sha1 = Sha1::new();
        sha1.update(&payload);
        let payload_sha1_hex = sha1.digest().to_string();
        let kfxgen_trailer = format!(
            "[{{key:\"kfxgen_package_version\",value:\"\"}},{{key:\"kfxgen_application_version\",value:\"ebook-rs\"}},{{key:\"kfxgen_payload_sha1\",value:\"{payload_sha1_hex}\"}},{{key:\"kfxgen_acr\",value:\"{container_id}\"}}]"
        );

        let header_len = info_offset + info_bytes.len() + kfxgen_trailer.len();
        let mut out = Vec::with_capacity(header_len + payload.len());
        out.extend_from_slice(KFX_MAGIC_CONT);
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&(header_len as u32).to_le_bytes());
        out.extend_from_slice(&(info_offset as u32).to_le_bytes());
        out.extend_from_slice(&(info_bytes.len() as u32).to_le_bytes());

        for entry in &entries {
            out.extend_from_slice(&entry.entity_id.to_le_bytes());
            out.extend_from_slice(&entry.type_id.to_le_bytes());
            out.extend_from_slice(&entry.offset.to_le_bytes());
            out.extend_from_slice(&entry.length.to_le_bytes());
        }

        out.extend_from_slice(&doc_symbol_bytes);
        out.extend_from_slice(&fcap_bytes);
        out.extend_from_slice(&info_bytes);
        out.extend_from_slice(kfxgen_trailer.as_bytes());
        debug_assert_eq!(out.len(), header_len);
        out.extend_from_slice(&payload);

        Ok(out)
    }
}
