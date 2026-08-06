use roxmltree::Document;
use serde::{Deserialize, Serialize};

/// Normal Play Time (NPT) clock helper for SMIL time strings (e.g., "00:01:23.45", "83.45s", "1:23.45").
pub struct SmilClock;

impl SmilClock {
    /// Parse SMIL NPT time string into seconds as f64.
    pub fn parse_npt_seconds(npt_str: &str) -> f64 {
        let clean = npt_str.trim().trim_end_matches('s').trim_end_matches('h');
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

/// Text target reference inside a SMIL parallel node (<text src="...">).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmilTextRef {
    pub src: String,
    pub full_path: String,
    pub element_id: Option<String>,
}

/// Audio clip payload inside a SMIL parallel node (<audio src="..." clipBegin="..." clipEnd="...">).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmilAudioClip {
    pub src: String,
    pub full_path: String,
    pub clip_begin: f64,
    pub clip_end: f64,
}

/// A synchronized parallel pair element (<par>).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaOverlayParallel {
    pub id: Option<String>,
    pub text: Option<SmilTextRef>,
    pub audio: Option<SmilAudioClip>,
}

/// A SMIL sequence container (<seq>).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaOverlaySequence {
    pub id: Option<String>,
    pub epub_textref: Option<String>,
    pub parallels: Vec<MediaOverlayParallel>,
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
        let doc = Document::parse(xml_content)
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
                    if (audio.src.ends_with(clean_audio) || audio.full_path.ends_with(clean_audio))
                        && timestamp_secs >= audio.clip_begin
                        && timestamp_secs <= audio.clip_end
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
