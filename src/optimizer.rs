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
        let mut report = OptimizationReport {
            original_size_bytes: 0,
            optimized_size_bytes: 0,
            saved_bytes: 0,
            deduplicated_assets_count: 0,
            purged_css_rules_count: 0,
        };

        // Measure the input first: `saved_bytes` and friends were never written by anything in
        // the crate, so every caller reading this report showed "0 bytes -> 0 bytes, saved 0"
        // no matter how much the optimizer actually removed.
        report.original_size_bytes = Self::total_size(book);

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
        // Recomputed after every stage so the figures reflect the bytes actually removed
        // (including asset deduplication, which shrinks the archive rather than the HTML).
        report.optimized_size_bytes = Self::total_size(book);
        report.saved_bytes = report
            .original_size_bytes
            .saturating_sub(report.optimized_size_bytes);
        report
    }

    /// Total on-disk footprint of a book: every archive entry plus the rendered section HTML.
    ///
    /// Both halves are counted because HTML minification changes one and asset deduplication
    /// changes the other.
    fn total_size(book: &Book) -> usize {
        let archive: usize = book.archive.files().values().map(Vec::len).sum();
        let sections: usize = book
            .sections
            .iter()
            .map(|s| s.raw_html.len() + s.processed_html.len())
            .sum();
        archive + sections
    }

    /// Minify HTML content by removing HTML comments and collapsing redundant whitespace.
    pub fn minify_html(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_tag = false;
        let mut in_quote: Option<char> = None;
        let mut in_pre = false;
        let bytes = html.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if !in_tag && html[i..].starts_with("<!--") {
                if let Some(end_idx) = html[i + 4..].find("-->") {
                    i += 4 + end_idx + 3;
                } else {
                    // Unterminated comment: `break` here silently discarded the entire remainder
                    // of the document. Emit the rest verbatim instead — a stray `<!--` in book
                    // HTML must not cost the reader the rest of the section.
                    out.push_str(&html[i..]);
                    break;
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
        let bytes = css.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if css[i..].starts_with("/*") {
                if let Some(end_idx) = css[i + 2..].find("*/") {
                    i += 2 + end_idx + 2;
                } else {
                    break;
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

        // Remap references to each removed asset and then drop it from the archive.
        if !path_redirects.is_empty() {
            let redirects: Vec<(String, String)> = path_redirects.into_iter().collect();
            for section in sections.iter_mut() {
                for (old_path, new_path) in &redirects {
                    section.raw_html = remap_asset_reference(&section.raw_html, old_path, new_path);
                    section.processed_html =
                        remap_asset_reference(&section.processed_html, old_path, new_path);
                }
            }
            remap_css_asset_references(archive, &redirects);
            for (old_path, _) in &redirects {
                archive.remove(old_path);
            }
        }

        dedup_count
    }
}

/// Rewrite references to `old_path` in HTML so they point at `new_path`.
///
/// Substitution is done on the **full path**, and only at positions that lie inside an HTML tag
/// — i.e. in an attribute value, including an inline `style="…url(…)…"`. The previous
/// implementation replaced bare filenames, which silently produced references to files that do
/// not exist whenever two identical assets live in different directories, and was a no-op while
/// still deleting the file when their basenames happened to match.
fn remap_asset_reference(html: &str, old_path: &str, new_path: &str) -> String {
    // Guard against one common case where a `#fragment`-style suffix follows the value.
    let inside_tag = tag_membership(html);
    remap_paths(html, old_path, new_path, |_, i| {
        inside_tag.get(i).copied().unwrap_or(false)
    })
}

/// Rewrite `url(...)` references in a CSS stylesheet so they point at `new_path`.
///
/// CSS has no tag context, so a reference is only rewritten when it sits inside a `url(...)`
/// token; arbitrary text that happens to contain the path is left alone.
fn remap_css_url_reference(css: &str, old_path: &str, new_path: &str) -> String {
    remap_paths(css, old_path, new_path, |text, i| {
        let before = text[..i].trim_end();
        let before = before.trim_end_matches(['"', '\'']);
        before.to_ascii_lowercase().ends_with("url(")
    })
}

/// Core substring rewriter: every whole-token occurrence of `old_path` that `allowed` accepts at
/// its start offset is replaced with `new_path`.
fn remap_paths(
    text: &str,
    old_path: &str,
    new_path: &str,
    allowed: impl Fn(&str, usize) -> bool,
) -> String {
    if old_path.is_empty() || old_path == new_path || !text.contains(old_path) {
        return text.to_string();
    }
    // Bytes that can terminate a path value in an attribute value or a `url(...)` token.
    const BREAK: &[u8] = b"\"')? #>;,&\t\r\n";
    let old = old_path.as_bytes();
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if bytes[i..].starts_with(old) {
            let next = i + old.len();
            let next_ok = next == bytes.len() || BREAK.contains(&bytes[next]);
            if next_ok && allowed(text, i) {
                out.push_str(new_path);
                i = next;
                continue;
            }
        }
        let n = text[i..].chars().next().map_or(1, char::len_utf8);
        out.push_str(&text[i..i + n]);
        i += n;
    }
    out
}

/// `mask[i]` is true when byte `i` of `text` lies inside an HTML tag, quote-aware.
///
/// Used to confine path rewriting to attribute values, so prose that merely mentions a filename
/// is never rewritten.
fn tag_membership(text: &str) -> Vec<bool> {
    let bytes = text.as_bytes();
    let mut mask = vec![false; bytes.len()];
    let mut in_tag = false;
    let mut quote: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !in_tag {
            if b == b'<' {
                in_tag = true;
                quote = None;
            }
        } else if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            in_tag = false;
        }
        // The `<` itself starts the tag, so it is part of it.
        mask[i] = in_tag;
        i += 1;
    }
    mask
}

/// Rewrite CSS `url(...)` references inside archive stylesheets.
///
/// Section HTML can reference an asset through CSS rather than through an attribute, so without
/// this step purging a duplicate stylesheet leaves every declaration pointing at a deleted file.
/// Unreadable or non-UTF-8 stylesheets are left untouched.
fn remap_css_asset_references(archive: &mut EpubArchive, redirects: &[(String, String)]) -> usize {
    if redirects.is_empty() {
        return 0;
    }
    let css_paths: Vec<String> = archive
        .list_files()
        .into_iter()
        .filter(|p| {
            let lower = p.to_lowercase();
            lower.ends_with(".css")
        })
        .collect();

    let mut remapped = 0;
    for path in css_paths {
        let Ok(text) = archive.read_string(&path) else {
            continue;
        };
        let mut updated = text.clone();
        for (old_path, new_path) in redirects {
            updated = remap_css_url_reference(&updated, old_path, new_path);
        }
        if updated != text {
            archive.insert(path, updated.into_bytes());
            remapped += 1;
        }
    }
    remapped
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Deduplication used to substitute the *basename*, so two identical images in different
    /// directories produced an `src` pointing at a file that does not exist, and identical
    /// basenames made the substitution a no-op while the file was still deleted.
    #[test]
    fn remap_asset_reference_rewrites_full_paths() {
        let html = r#"<img src="images/logo.png"><img src="cover.png" href="x">"#;
        let out = remap_asset_reference(html, "images/logo.png", "cover.png");
        assert_eq!(
            out,
            r#"<img src="cover.png"><img src="cover.png" href="x">"#
        );
        // Unquoted and single-quoted forms.
        assert_eq!(
            remap_asset_reference("<img src=images/logo.png>", "images/logo.png", "cover.png"),
            "<img src=cover.png>"
        );
        assert_eq!(
            remap_asset_reference(
                "<img src=@'images/logo.png'@>".replace('@', "").as_str(),
                "images/logo.png",
                "cover.png"
            ),
            "<img src='cover.png'>"
        );
        // Inline CSS url().
        assert_eq!(
            remap_asset_reference(
                r#"<div style="background:url(images/logo.png)"></div>"#,
                "images/logo.png",
                "cover.png"
            ),
            r#"<div style="background:url(cover.png)"></div>"#
        );
        assert_eq!(
            remap_asset_reference(
                r#"<div style="background:url('images/logo.png')"></div>"#,
                "images/logo.png",
                "cover.png"
            ),
            r#"<div style="background:url('cover.png')"></div>"#
        );
        // Text content that merely contains the path must not be touched.
        assert_eq!(
            remap_asset_reference(
                "<p>See images/logo.png for details</p>",
                "images/logo.png",
                "cover.png"
            ),
            "<p>See images/logo.png for details</p>"
        );
        // A prefix of the path must not be partially rewritten.
        assert_eq!(
            remap_asset_reference(
                r#"<img src="images/logo.png.bak">"#,
                "images/logo.png",
                "cover.png"
            ),
            r#"<img src="images/logo.png.bak">"#
        );
    }

    /// An unterminated `<!--` used to `break` out of the loop, silently throwing away the rest
    /// of the document.
    #[test]
    fn minify_html_keeps_tail_of_unterminated_comment() {
        let out = EpubOptimizer::minify_html(
            "<p>keep this</p><!-- unterminated comment <p>secret tail</p>",
        );
        assert!(out.contains("keep this"), "{out}");
        assert!(out.contains("secret tail"), "tail was discarded: {out}");
    }

    #[test]
    fn minify_html_still_removes_comments() {
        assert_eq!(
            EpubOptimizer::minify_html("<p>a</p><!-- gone --><p>b</p>"),
            "<p>a</p><p>b</p>"
        );
    }
}
