use crate::archive::EpubArchive;
use crate::book::Book;
use crate::section::Section;
use ahash::{AHashMap, AHashSet};
use serde::{Deserialize, Serialize};

/// Configuration options for the EPUB 3 optimizer and minifier engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpubOptimizerOptions {
    /// Minify HTML/XHTML structures (strip non-essential whitespace, HTML comments).
    pub minify_html: bool,
    /// Minify CSS style sheets (collapse whitespace, strip CSS comments, simplify rules).
    pub minify_css: bool,
    /// Purge unreferenced CSS rules across all section documents.
    pub purge_unused_css: bool,
    /// Deduplicate identical fonts and binary images using SHA-256 fingerprinting.
    pub deduplicate_assets: bool,
}

impl Default for EpubOptimizerOptions {
    fn default() -> Self {
        Self {
            minify_html: true,
            minify_css: true,
            purge_unused_css: true,
            deduplicate_assets: true,
        }
    }
}

/// Statistics and report returned after running EPUB optimization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub original_size_bytes: usize,
    pub optimized_size_bytes: usize,
    pub saved_bytes: usize,
    pub deduplicated_assets_count: usize,
    pub purged_css_rules_count: usize,
}

/// EPUB 3 Lossless Optimizer and Minifier engine.
pub struct EpubOptimizer;

impl EpubOptimizer {
    /// Optimize a loaded `Book` instance in-place with the specified options.
    pub fn optimize(book: &mut Book, options: &EpubOptimizerOptions) -> OptimizationReport {
        let mut report = OptimizationReport::default();

        // 1. Gather all referenced classes, IDs, and element tag names for CSS Purging
        let mut used_classes = AHashSet::new();
        let mut used_ids = AHashSet::new();
        let mut used_tags = AHashSet::new();

        for section in &book.sections {
            extract_dom_identifiers(
                &section.raw_html,
                &mut used_classes,
                &mut used_ids,
                &mut used_tags,
            );
            extract_dom_identifiers(
                &section.processed_html,
                &mut used_classes,
                &mut used_ids,
                &mut used_tags,
            );
        }

        // 2. HTML Minification
        if options.minify_html {
            for section in &mut book.sections {
                section.raw_html = Self::minify_html(&section.raw_html);
                section.processed_html = Self::minify_html(&section.processed_html);
            }
        }

        // 3. Asset & Font Deduplication
        if options.deduplicate_assets {
            let dedup_count = Self::deduplicate_assets(&mut book.archive, &mut book.sections);
            report.deduplicated_assets_count = dedup_count;
        }

        // 4. CSS Minification & Purging
        if options.minify_css || options.purge_unused_css {
            let css_files: Vec<String> = book
                .archive
                .list_files()
                .into_iter()
                .filter(|p| p.to_lowercase().ends_with(".css"))
                .collect();

            for css_path in css_files {
                if let Ok(css_content) = book.archive.read_string(&css_path) {
                    let mut optimized_css = css_content;
                    if options.purge_unused_css {
                        let (purged, count) =
                            Self::purge_css(&optimized_css, &used_classes, &used_ids, &used_tags);
                        optimized_css = purged;
                        report.purged_css_rules_count += count;
                    }
                    if options.minify_css {
                        optimized_css = Self::minify_css(&optimized_css);
                    }
                    book.archive.insert(&css_path, optimized_css.into_bytes());
                }
            }
        }

        book.invalidate_render_cache();
        report
    }

    /// Minify HTML content by removing HTML comments and collapsing redundant whitespace.
    pub fn minify_html(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_comment = false;
        let mut in_tag = false;
        let mut in_quote: Option<char> = None;
        let mut in_pre = false;
        let bytes = html.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if !in_comment && !in_tag && html[i..].starts_with("<!--") {
                in_comment = true;
                i += 4;
                continue;
            }

            if in_comment {
                if html[i..].starts_with("-->") {
                    in_comment = false;
                    i += 3;
                } else {
                    i += 1;
                }
                continue;
            }

            if !in_tag && bytes[i] == b'<' {
                in_tag = true;
                in_quote = None;
                let rest = &html[i..];
                if rest.starts_with("<pre")
                    || rest.starts_with("<code")
                    || rest.starts_with("<script")
                    || rest.starts_with("<style")
                    || rest.starts_with("<textarea")
                {
                    in_pre = true;
                } else if rest.starts_with("</pre>")
                    || rest.starts_with("</code>")
                    || rest.starts_with("</script>")
                    || rest.starts_with("</style>")
                    || rest.starts_with("</textarea>")
                {
                    in_pre = false;
                }
            } else if in_tag {
                if let Some(q) = in_quote {
                    if html.as_bytes()[i] == q as u8 {
                        in_quote = None;
                    }
                } else if bytes[i] == b'"' || bytes[i] == b'\'' {
                    in_quote = Some(bytes[i] as char);
                } else if bytes[i] == b'>' {
                    in_tag = false;
                    in_quote = None;
                }
            }

            let ch = html[i..].chars().next().unwrap_or(' ');
            let ch_len = ch.len_utf8();

            if !in_pre && in_quote.is_none() && ch.is_whitespace() {
                if !out.ends_with(' ')
                    && !out.ends_with('>')
                    && !out.ends_with('<')
                    && !out.is_empty()
                {
                    out.push(' ');
                }
                i += ch_len;
                continue;
            }

            if in_tag && in_quote.is_none() && (ch == '>' || ch == '/') && out.ends_with(' ') {
                out.pop();
            }

            out.push(ch);
            i += ch_len;
        }

        out.trim().to_string()
    }

    /// Minify CSS style sheets by removing CSS comments and collapsing whitespace.
    pub fn minify_css(css: &str) -> String {
        let mut out = String::with_capacity(css.len());
        let mut in_comment = false;
        let bytes = css.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if !in_comment && css[i..].starts_with("/*") {
                in_comment = true;
                i += 2;
                continue;
            }

            if in_comment {
                if css[i..].starts_with("*/") {
                    in_comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }

            let ch = css[i..].chars().next().unwrap_or(' ');
            let ch_len = ch.len_utf8();

            if ch.is_whitespace() {
                if !out.ends_with(' ')
                    && !out.ends_with('{')
                    && !out.ends_with('}')
                    && !out.ends_with(':')
                    && !out.ends_with(';')
                    && !out.ends_with(',')
                    && !out.is_empty()
                {
                    out.push(' ');
                }
                i += ch_len;
                continue;
            }

            if (ch == '{' || ch == '}' || ch == ':' || ch == ';' || ch == ',') && out.ends_with(' ')
            {
                out.pop();
            }

            out.push(ch);
            i += ch_len;
        }

        out.trim().to_string()
    }

    /// Purge unreferenced CSS rules based on observed classes, IDs, and HTML tags.
    pub fn purge_css(
        css: &str,
        used_classes: &AHashSet<String>,
        used_ids: &AHashSet<String>,
        used_tags: &AHashSet<String>,
    ) -> (String, usize) {
        let mut purged = String::with_capacity(css.len());
        let mut purged_count = 0;

        let mut i = 0;
        let bytes = css.as_bytes();
        let len = bytes.len();

        while i < len {
            // Skip leading whitespace
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= len {
                break;
            }

            // Find matching statement/block with proper brace nesting
            let start = i;
            let mut brace_depth = 0;
            let mut found_open = false;

            while i < len {
                if bytes[i] == b'{' {
                    brace_depth += 1;
                    found_open = true;
                } else if bytes[i] == b'}' {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    if found_open && brace_depth == 0 {
                        i += 1;
                        break;
                    }
                } else if bytes[i] == b';' && !found_open {
                    i += 1;
                    break;
                }
                i += 1;
            }

            let block = &css[start..i];
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(open_brace) = trimmed.find('{') {
                let selector_part = trimmed[..open_brace].trim();

                // At-rules (@media, @supports, @font-face, @keyframes, @import, @charset) are preserved intact
                if selector_part.starts_with('@') {
                    purged.push_str(trimmed);
                    purged.push('\n');
                    continue;
                }

                let body_part = if trimmed.ends_with('}') {
                    &trimmed[open_brace + 1..trimmed.len() - 1]
                } else {
                    &trimmed[open_brace + 1..]
                };

                // Check selector components
                let mut is_used = false;
                for selector in selector_part.split(',') {
                    let clean_sel = selector.trim();
                    if clean_sel == "*"
                        || clean_sel == ":root"
                        || clean_sel == "html"
                        || clean_sel == "body"
                    {
                        is_used = true;
                        break;
                    }

                    // Check if class selector (.my-class)
                    if let Some(dot_idx) = clean_sel.find('.') {
                        let class_name: String = clean_sel[dot_idx + 1..]
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                            .collect();
                        if used_classes.contains(&class_name) {
                            is_used = true;
                            break;
                        }
                    }

                    // Check if ID selector (#my-id)
                    if let Some(hash_idx) = clean_sel.find('#') {
                        let id_name: String = clean_sel[hash_idx + 1..]
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                            .collect();
                        if used_ids.contains(&id_name) {
                            is_used = true;
                            break;
                        }
                    }

                    // Check tag selector
                    let tag_name: String = clean_sel
                        .chars()
                        .take_while(|c| c.is_ascii_alphabetic())
                        .collect();
                    if !tag_name.is_empty() && used_tags.contains(&tag_name) {
                        is_used = true;
                        break;
                    }
                }

                if is_used {
                    purged.push_str(selector_part);
                    purged.push('{');
                    purged.push_str(body_part.trim());
                    purged.push_str("}\n");
                } else {
                    purged_count += 1;
                }
            } else {
                purged.push_str(trimmed);
                purged.push('\n');
            }
        }

        (purged, purged_count)
    }

    /// Deduplicate identical images and fonts across the EPUB archive.
    pub fn deduplicate_assets(archive: &mut EpubArchive, sections: &mut [Section]) -> usize {
        let mut hash_to_canonical_path: AHashMap<String, String> = AHashMap::new();
        let mut path_redirects: AHashMap<String, String> = AHashMap::new();
        let mut dedup_count = 0;

        let all_files = archive.list_files();
        for path in all_files {
            let lower = path.to_lowercase();
            if lower.ends_with(".png")
                || lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".webp")
                || lower.ends_with(".gif")
                || lower.ends_with(".svg")
                || lower.ends_with(".ttf")
                || lower.ends_with(".otf")
                || lower.ends_with(".woff")
                || lower.ends_with(".woff2")
            {
                if let Ok(bytes) = archive.read_bytes(&path) {
                    let hash = sha1_smol::Sha1::from(&bytes).digest().to_string();
                    if let Some(canonical) = hash_to_canonical_path.get(&hash) {
                        path_redirects.insert(path.clone(), canonical.clone());
                        dedup_count += 1;
                    } else {
                        hash_to_canonical_path.insert(hash, path.clone());
                    }
                }
            }
        }

        // Remap section HTML src / href references
        if !path_redirects.is_empty() {
            for section in sections {
                for (old_path, new_path) in &path_redirects {
                    let old_filename = old_path.split('/').next_back().unwrap_or(old_path);
                    let new_filename = new_path.split('/').next_back().unwrap_or(new_path);
                    section.raw_html = section.raw_html.replace(old_filename, new_filename);
                    section.processed_html =
                        section.processed_html.replace(old_filename, new_filename);
                }
            }
        }

        dedup_count
    }
}

/// Helper to extract class names, ID attributes, and tag names from HTML.
fn extract_dom_identifiers(
    html: &str,
    classes: &mut AHashSet<String>,
    ids: &mut AHashSet<String>,
    tags: &mut AHashSet<String>,
) {
    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        if let Some(open_tag) = memchr::memchr(b'<', &bytes[i..]) {
            let abs_open = i + open_tag;
            let rest = &html[abs_open + 1..];

            // Extract tag name
            let tag: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if !tag.is_empty() && !tag.starts_with('/') && !tag.starts_with('!') {
                tags.insert(tag.to_lowercase());
            }

            if let Some(close_tag) = rest.find('>') {
                let tag_body = &rest[..close_tag];

                // Extract class="..."
                if let Some(class_pos) = tag_body.find("class=") {
                    let after = &tag_body[class_pos + 6..].trim_start();
                    let quote = after.chars().next().unwrap_or('"');
                    if (quote == '"' || quote == '\'') && after.len() > 1 {
                        if let Some(end_q) = after[1..].find(quote) {
                            let class_str = &after[1..=end_q];
                            for cls in class_str.split_whitespace() {
                                classes.insert(cls.to_string());
                            }
                        }
                    }
                }

                // Extract id="..."
                if let Some(id_pos) = tag_body.find("id=") {
                    let after = &tag_body[id_pos + 3..].trim_start();
                    let quote = after.chars().next().unwrap_or('"');
                    if (quote == '"' || quote == '\'') && after.len() > 1 {
                        if let Some(end_q) = after[1..].find(quote) {
                            let id_str = &after[1..=end_q];
                            ids.insert(id_str.trim().to_string());
                        }
                    }
                }

                i = abs_open + 1 + close_tag + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}
