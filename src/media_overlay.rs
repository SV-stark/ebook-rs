use serde::{Deserialize, Serialize};

/// Rewrite a matched `id="…"` span into full karaoke-annotation attributes.
///
/// Looks at the enclosing tag first: if it already declares a `class`, the highlight class is
/// appended inside that attribute rather than emitted as a second, ignored one.
fn rewrite_overlay_tag(
    html: &mut String,
    target: &str,
    cue: &WebAudioCue,
    quote: char,
    elem_id: &str,
) {
    let Some(id_at) = html.find(target) else {
        return;
    };
    // Walk back to the tag that owns this attribute.
    let tag_start = html[..id_at].rfind('<').unwrap_or(0);
    let tag_end = {
        let bytes = html.as_bytes();
        let mut i = id_at;
        let mut q: Option<u8> = None;
        while i < bytes.len() {
            let b = bytes[i];
            match q {
                Some(open) => {
                    if b == open {
                        q = None;
                    }
                }
                None => {
                    if b == b'"' || b == b'\'' {
                        q = Some(b);
                    } else if b == b'>' {
                        break;
                    }
                }
            }
            i += 1;
        }
        i.min(html.len())
    };
    let tag = &html[tag_start..tag_end];
    let id_attr = format!("id={quote}{elem_id}{quote}");

    let data = format!(
        " data-audio-src=\"{}\" data-clip-begin=\"{:.3}\" data-clip-end=\"{:.3}\"",
        cue.audio_src, cue.clip_begin, cue.clip_end
    );

    // Case-insensitive lookup of an existing `class` attribute on this tag.
    let class_value = find_attribute(tag, "class");

    let rewritten = match class_value {
        Some((start, quote_ch)) => {
            let close = tag[start..]
                .find(quote_ch)
                .map(|p| start + p)
                .unwrap_or(tag.len());
            let mut merged = String::with_capacity(tag.len() + 32);
            merged.push_str(&tag[..close]);
            if !merged.ends_with(' ') {
                merged.push(' ');
            }
            merged.push_str("media-overlay-active-target");
            merged.push_str(&tag[close..]);
            merged
        }
        None => {
            let at = tag.find(&id_attr).unwrap_or(0);
            let insert_at = at + id_attr.len();
            format!(
                "{}{} class=\"media-overlay-active-target\"",
                &tag[..insert_at],
                data
            ) + &tag[insert_at..]
        }
    };
    html.replace_range(tag_start..tag_end, "");
    html.insert_str(tag_start, &rewritten);
}

/// Find an attribute's value span inside a single tag's text.
///
/// Returns the offset just past the opening quote and the quote character used, so the caller can
/// locate the closing quote. Matching is ASCII case-insensitive, like HTML attribute names.
fn find_attribute(tag: &str, name: &str) -> Option<(usize, char)> {
    let bytes = tag.as_bytes();
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let mut from = 0;
    while let Some(rel) = lower[from..].find(&needle) {
        let at = from + rel;
        // Must be preceded by whitespace, the tag name, or a `/`.
        let boundary =
            at == 0 || matches!(bytes[at - 1], b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'<');
        if boundary {
            let after = at + needle.len();
            if let Some(ch) = tag[after..].chars().next() {
                if ch == '"' || ch == '\'' {
                    return Some((after + ch.len_utf8(), ch));
                }
            }
        }
        from = at + needle.len();
    }
    None
}

/// Normal Play Time (NPT) clock helper for SMIL time strings (e.g., "00:01:23.45", "83.45s", "1:23.45").
pub struct SmilClock;

impl SmilClock {
    /// Parse SMIL NPT time string into seconds as f64.
    ///
    /// SMIL 3.0 timecount values are `h`, `min`, `s` and `ms`. The unit suffix used to be
    /// detected by inspecting the *raw* last character while the value was stripped with
    /// `trim_end_matches('s' | 'h' | 'm')`; a single-character strip cannot remove `min` or `ms`,
    /// so `90min` left `90mi` unparseable (→ `0.0`) and `250ms` left `250m` (→ `250.0`, i.e. a
    /// quarter-second clip became four minutes). Match on the suffix instead.
    pub fn parse_npt_seconds(npt_str: &str) -> f64 {
        let trimmed = npt_str.trim().trim_start_matches("npt=");

        if let Some(rest) = trimmed.strip_suffix("ms") {
            return rest.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
        }
        if let Some(rest) = trimmed.strip_suffix("min") {
            return rest.trim().parse::<f64>().unwrap_or(0.0) * 60.0;
        }
        if let Some(rest) = trimmed.strip_suffix('h') {
            return rest.trim().parse::<f64>().unwrap_or(0.0) * 3600.0;
        }
        if let Some(rest) = trimmed.strip_suffix("s") {
            return rest.trim().parse::<f64>().unwrap_or(0.0);
        }
        // A bare `m` suffix is minutes (`1.5m`); `min` and `ms` are already handled above.
        if let Some(rest) = trimmed.strip_suffix('m') {
            return rest.trim().parse::<f64>().unwrap_or(0.0) * 60.0;
        }

        let clean = trimmed.trim_end_matches('s');
        if clean.contains(':') {
            let parts: Vec<&str> = clean.split(':').collect();
            match parts.len() {
                2 => {
                    let mins: f64 = parts[0].parse().unwrap_or(0.0);
                    let secs: f64 = parts[1].parse().unwrap_or(0.0);
                    mins * 60.0 + secs
                }
                3 => {
                    let hours: f64 = parts[0].parse().unwrap_or(0.0);
                    let mins: f64 = parts[1].parse().unwrap_or(0.0);
                    let secs: f64 = parts[2].parse().unwrap_or(0.0);
                    hours * 3600.0 + mins * 60.0 + secs
                }
                _ => 0.0,
            }
        } else {
            clean.parse::<f64>().unwrap_or(0.0)
        }
    }

    /// Format seconds as NPT clock string ("00:00:00.000").
    pub fn format_npt(seconds: f64) -> String {
        let total_ms = (seconds * 1000.0).round() as u64;
        let hours = total_ms / 3_600_000;
        let mins = (total_ms % 3_600_000) / 60_000;
        let secs = (total_ms % 60_000) / 1000;
        let ms = total_ms % 1000;
        format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
    }
}

/// Text target reference inside a SMIL parallel node (`<text src="...">`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmilTextRef {
    pub src: String,
    pub full_path: String,
    pub element_id: Option<String>,
}

/// Audio clip payload inside a SMIL parallel node (`<audio src="..." clipBegin="..." clipEnd="...">`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmilAudioClip {
    pub src: String,
    pub full_path: String,
    pub clip_begin: f64,
    pub clip_end: f64,
}

/// A synchronized parallel pair element (`<par>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaOverlayParallel {
    pub id: Option<String>,
    pub text: Option<SmilTextRef>,
    pub audio: Option<SmilAudioClip>,
}

/// A SMIL sequence container (`<seq>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaOverlaySequence {
    pub id: Option<String>,
    pub epub_textref: Option<String>,
    pub parallels: Vec<MediaOverlayParallel>,
}

/// A Web Audio API / Karaoke cue representation for real-time audio playback synchronization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebAudioCue {
    pub id: String,
    pub element_id: Option<String>,
    pub text_src: String,
    pub audio_src: String,
    pub clip_begin: f64,
    pub clip_end: f64,
    pub duration: f64,
}

/// Complete synchronized karaoke cue sheet for Web Audio API players.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KaraokeCueSheet {
    pub smil_path: String,
    pub total_duration: f64,
    pub cues: Vec<WebAudioCue>,
}

/// Parsed SMIL 3.0 Media Overlay Package for an EPUB section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaOverlayPackage {
    pub id: String,
    pub smil_path: String,
    pub sequences: Vec<MediaOverlaySequence>,
}

impl MediaOverlayPackage {
    /// Parse SMIL XML content into a `MediaOverlayPackage`.
    pub fn parse_smil(xml_content: &str, smil_path: &str) -> Result<Self, String> {
        let doc = crate::dom::parse_xml_document(xml_content)
            .map_err(|e| format!("XML parse error in SMIL file {}: {}", smil_path, e))?;

        let smil_dir = if let Some(idx) = smil_path.rfind('/') {
            &smil_path[..idx]
        } else {
            ""
        };

        let mut sequences = Vec::new();

        for node in doc.descendants() {
            if node.has_tag_name("seq") {
                let seq_id = node.attribute("id").map(|s| s.to_string());
                let textref = node
                    .attribute(("http://www.idpf.org/2007/ops", "textref"))
                    .or_else(|| node.attribute("epub:textref"))
                    .or_else(|| node.attribute("textref"))
                    .map(|s| s.to_string());

                let mut parallels = Vec::new();

                for child in node.children() {
                    if child.has_tag_name("par") {
                        let par_id = child.attribute("id").map(|s| s.to_string());
                        let mut text_ref = None;
                        let mut audio_clip = None;

                        for par_child in child.children() {
                            if par_child.has_tag_name("text") {
                                if let Some(src) = par_child.attribute("src") {
                                    let full_path =
                                        crate::archive::resolve_relative_path(smil_dir, src);
                                    let element_id = src.split('#').nth(1).map(|s| s.to_string());
                                    text_ref = Some(SmilTextRef {
                                        src: src.to_string(),
                                        full_path,
                                        element_id,
                                    });
                                }
                            } else if par_child.has_tag_name("audio") {
                                if let Some(src) = par_child.attribute("src") {
                                    let full_path =
                                        crate::archive::resolve_relative_path(smil_dir, src);
                                    let clip_begin = par_child
                                        .attribute("clipBegin")
                                        .map(SmilClock::parse_npt_seconds)
                                        .unwrap_or(0.0);
                                    let clip_end = par_child
                                        .attribute("clipEnd")
                                        .map(SmilClock::parse_npt_seconds)
                                        .unwrap_or(0.0);

                                    audio_clip = Some(SmilAudioClip {
                                        src: src.to_string(),
                                        full_path,
                                        clip_begin,
                                        clip_end,
                                    });
                                }
                            }
                        }

                        if text_ref.is_some() || audio_clip.is_some() {
                            parallels.push(MediaOverlayParallel {
                                id: par_id,
                                text: text_ref,
                                audio: audio_clip,
                            });
                        }
                    }
                }

                if !parallels.is_empty() {
                    sequences.push(MediaOverlaySequence {
                        id: seq_id,
                        epub_textref: textref,
                        parallels,
                    });
                }
            }
        }

        Ok(Self {
            id: smil_path.to_string(),
            smil_path: smil_path.to_string(),
            sequences,
        })
    }

    /// Convert SMIL sequences into a flat vector of `WebAudioCue` entries for Web Audio API players.
    pub fn to_karaoke_cues(&self) -> Vec<WebAudioCue> {
        let mut cues = Vec::new();
        let mut cue_idx = 0;

        for seq in &self.sequences {
            for par in &seq.parallels {
                if let (Some(text), Some(audio)) = (&par.text, &par.audio) {
                    let id = par.id.clone().unwrap_or_else(|| format!("cue_{}", cue_idx));
                    let duration = (audio.clip_end - audio.clip_begin).max(0.0);
                    cues.push(WebAudioCue {
                        id,
                        element_id: text.element_id.clone(),
                        text_src: text.src.clone(),
                        audio_src: audio.full_path.clone(),
                        clip_begin: audio.clip_begin,
                        clip_end: audio.clip_end,
                        duration,
                    });
                    cue_idx += 1;
                }
            }
        }

        cues
    }

    /// Build a complete `KaraokeCueSheet` ready for Web Audio API audio player dispatch.
    pub fn to_karaoke_cue_sheet(&self) -> KaraokeCueSheet {
        let cues = self.to_karaoke_cues();
        let total_duration = cues
            .iter()
            .map(|c| c.clip_end)
            .fold(0.0f64, |acc, val| acc.max(val));

        KaraokeCueSheet {
            smil_path: self.smil_path.clone(),
            total_duration,
            cues,
        }
    }

    /// Annotate raw HTML with Web Audio API karaoke attributes (`data-audio-src`, `data-clip-begin`,
    /// `data-clip-end`, `data-overlay-active`).
    ///
    /// A highlight class is only injected when the matched tag does not already declare one.
    /// HTML5 honours the *first* `class` attribute on an element, so emitting a second one after
    /// the `id` silently discarded the author's own class list.
    pub fn annotate_html_with_media_overlays(&self, html: &str) -> String {
        let cues = self.to_karaoke_cues();
        if cues.is_empty() {
            return html.to_string();
        }

        let mut output = html.to_string();
        for cue in cues {
            if let Some(ref elem_id) = cue.element_id {
                for quote in ['"', '\''] {
                    let target_pattern = format!("id={quote}{elem_id}{quote}");
                    if !output.contains(&target_pattern) {
                        continue;
                    }
                    rewrite_overlay_tag(&mut output, &target_pattern, &cue, quote, elem_id);
                }
            }
        }

        output
    }

    /// Generate JSON manifest for Web Audio API playback synchronization.
    pub fn generate_web_audio_manifest(&self) -> Result<String, String> {
        let cue_sheet = self.to_karaoke_cue_sheet();
        serde_json::to_string_pretty(&cue_sheet)
            .map_err(|e| format!("Failed to serialize Web Audio karaoke cue sheet: {}", e))
    }

    /// Lookup audio clip timing for a specific text target href or element ID.
    pub fn find_audio_clip_by_text_href(&self, href: &str) -> Option<SmilAudioClip> {
        let clean = href.trim();
        for seq in &self.sequences {
            for par in &seq.parallels {
                if let Some(ref text) = par.text {
                    if text.src == clean
                        || text.full_path == clean
                        || text.src.ends_with(clean)
                        || text.element_id.as_deref() == Some(clean)
                    {
                        return par.audio.clone();
                    }
                }
            }
        }
        None
    }

    /// Lookup text target element for a specific audio timestamp in seconds.
    pub fn find_text_ref_by_timestamp(
        &self,
        audio_path: &str,
        timestamp_secs: f64,
    ) -> Option<SmilTextRef> {
        let clean_audio = audio_path.trim();
        for seq in &self.sequences {
            for par in &seq.parallels {
                if let Some(ref audio) = par.audio {
                    let clip_end = if audio.clip_end > 0.0 {
                        audio.clip_end
                    } else {
                        f64::MAX
                    };

                    if (audio.src == clean_audio
                        || audio.full_path == clean_audio
                        || audio.src.ends_with(clean_audio))
                        && timestamp_secs >= audio.clip_begin
                        && timestamp_secs <= clip_end
                    {
                        return par.text.clone();
                    }
                }
            }
        }
        None
    }

    /// Serialize SMIL Media Overlay Package to JSON string.
    pub fn to_smil_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize SMIL Media Overlay to JSON: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_cue_package() -> MediaOverlayPackage {
        let smil = r#"<?xml version="1.0"?>
<smil xmlns="http://www.w3.org/ns/SMIL" version="3.0"><body><seq id="s1"><par id="p1"><text src="page.xhtml#p1"/><audio src="a.mp3" clipBegin="0:00:01.500" clipEnd="0:00:05.200"/></par></seq></body></smil>"#;
        MediaOverlayPackage::parse_smil(smil, "OEBPS/media/ch1.smil").unwrap()
    }

    /// The class used to be emitted a second time right after the `id` attribute, and HTML5 keeps
    /// only the first `class` on an element, so the author's own class was silently dropped.
    #[test]
    fn highlight_class_merges_into_an_existing_class_attribute() {
        let p = two_cue_package();
        let html = r#"<div class="chapter"><p id="p1" class="intro">First</p></div>"#;
        let out = p.annotate_html_with_media_overlays(html);
        assert!(
            out.contains(r#"class="intro media-overlay-active-target""#),
            "author class was dropped: {out}"
        );
        assert_eq!(out.matches("media-overlay-active-target").count(), 1);
        // The sibling element's own class must be left alone.
        assert!(out.contains(r#"<div class="chapter">"#), "{out}");
    }

    #[test]
    fn highlight_class_is_added_when_absent() {
        let p = two_cue_package();
        let html = r#"<p id="p1">First</p>"#;
        let out = p.annotate_html_with_media_overlays(html);
        assert!(
            out.contains(r#"class="media-overlay-active-target""#),
            "{out}"
        );
        assert_eq!(out.matches("media-overlay-active-target").count(), 1);
        assert!(out.contains(r#"id="p1""#), "{out}");
    }

    /// SMIL 3.0 timecount units are `h`, `min`, `s` and `ms`. A single-character suffix strip
    /// cannot remove `min` or `ms`, so `90min` used to parse as `0.0` and `250ms` as `250.0`
    /// (a quarter-second clip became four minutes).
    #[test]
    fn smil_clock_parses_all_timecount_units() {
        assert_eq!(SmilClock::parse_npt_seconds("10s"), 10.0);
        assert_eq!(SmilClock::parse_npt_seconds("2.5s"), 2.5);
        assert_eq!(SmilClock::parse_npt_seconds("100ms"), 0.1);
        assert_eq!(SmilClock::parse_npt_seconds("250ms"), 0.25);
        assert_eq!(SmilClock::parse_npt_seconds("90min"), 5400.0);
        assert_eq!(SmilClock::parse_npt_seconds("1.5m"), 90.0);
        assert_eq!(SmilClock::parse_npt_seconds("2h"), 7200.0);
        assert_eq!(SmilClock::parse_npt_seconds("01:30"), 90.0);
        assert_eq!(SmilClock::parse_npt_seconds("01:02:03"), 3723.0);
        // Unparseable values degrade to zero rather than panicking.
        assert_eq!(SmilClock::parse_npt_seconds("not-a-time"), 0.0);
    }
}
