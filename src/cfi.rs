use serde::{Deserialize, Serialize};

/// A parsed EPUB Canonical Fragment Identifier (CFI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cfi {
    pub raw: String,
    pub path: CfiPath,
    pub range_start: Option<CfiPath>,
    pub range_end: Option<CfiPath>,
}

/// DOM Element target resolved from CFI element steps and ID assertions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfiDomTarget {
    pub element_id: Option<String>,
    pub css_selector: String,
    pub char_offset: usize,
}

/// A path component inside a CFI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CfiPath {
    pub steps: Vec<CfiStep>,
    pub offset: Option<CfiOffset>,
}

/// A single step in a CFI path (e.g. /6/4[chap01ref]!).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CfiStep {
    pub index: usize,
    pub indirection: bool,
    pub element_id: Option<String>,
}

/// Character or temporal offset at the end of a CFI path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CfiOffset {
    Character(usize),
    Temporal(f64),
    Spatial(f64, f64),
}

impl Cfi {
    /// Parse a CFI string representation into a structured `Cfi`.
    pub fn parse(cfi_str: &str) -> Result<Self, String> {
        let clean = cfi_str.trim();
        let payload = if clean.starts_with("epubcfi(") && clean.ends_with(')') {
            &clean[8..clean.len() - 1]
        } else {
            clean
        };

        if payload.contains(',') {
            // Range CFI: /6/2!/4/2/1, :10, :45
            let parts: Vec<&str> = payload.split(',').collect();
            if parts.len() < 3 {
                return Err("Invalid range CFI: expected 3 comma-separated components".to_string());
            }
            let parent_str = parts[0];
            let start_str = parts[1];
            let end_str = parts[2];

            let parent_path = parse_single_path(parent_str)?;
            let mut start_path = parse_single_path(start_str)?;
            let mut end_path = parse_single_path(end_str)?;

            // Combine parent steps into range endpoints
            let mut full_start_steps = parent_path.steps.clone();
            full_start_steps.append(&mut start_path.steps);
            let combined_start = CfiPath {
                steps: full_start_steps,
                offset: start_path.offset,
            };

            let mut full_end_steps = parent_path.steps.clone();
            full_end_steps.append(&mut end_path.steps);
            let combined_end = CfiPath {
                steps: full_end_steps,
                offset: end_path.offset,
            };

            Ok(Self {
                raw: cfi_str.to_string(),
                path: combined_start.clone(),
                range_start: Some(combined_start),
                range_end: Some(combined_end),
            })
        } else {
            let path = parse_single_path(payload)?;
            Ok(Self {
                raw: cfi_str.to_string(),
                path,
                range_start: None,
                range_end: None,
            })
        }
    }

    /// Helper constructor: Create a simple CFI pointing to a spine index and character offset.
    /// B5 Fix: Support optional element_id assertion to preserve CFI id assertions on roundtrip.
    pub fn from_spine_index(
        spine_index: usize,
        element_id: Option<&str>,
        char_offset: usize,
    ) -> Self {
        let spine_step_idx = (spine_index + 1) * 2;
        let steps = vec![
            CfiStep {
                index: 6,
                indirection: false,
                element_id: None,
            },
            CfiStep {
                index: spine_step_idx,
                indirection: true,
                element_id: element_id.map(|s| s.to_string()),
            },
            CfiStep {
                index: 4,
                indirection: false,
                element_id: None,
            },
            CfiStep {
                index: 2,
                indirection: false,
                element_id: None,
            },
            CfiStep {
                index: 1,
                indirection: false,
                element_id: None,
            },
        ];

        let id_str = element_id.map(|s| format!("[{}]", s)).unwrap_or_default();
        let raw = format!(
            "epubcfi(/6/{}{}!/4/2/1:{})",
            spine_step_idx, id_str, char_offset
        );
        let path = CfiPath {
            steps: steps.clone(),
            offset: Some(CfiOffset::Character(char_offset)),
        };

        Self {
            raw,
            path,
            range_start: None,
            range_end: None,
        }
    }

    /// Extract the 0-based spine item index from this CFI, returning an error if no indirection step `!` is found (B5 Fix).
    pub fn try_spine_index(&self) -> Result<usize, String> {
        for step in &self.path.steps {
            if step.indirection && step.index >= 2 {
                return Ok((step.index / 2) - 1);
            }
        }
        Err("CFI missing indirection step ('!')".to_string())
    }

    /// Extract the 0-based spine item index from this CFI (defaults to 0 if missing indirection).
    pub fn spine_index(&self) -> usize {
        self.try_spine_index().unwrap_or(0)
    }

    /// Extract character offset.
    pub fn char_offset(&self) -> usize {
        match self.path.offset {
            Some(CfiOffset::Character(off)) => off,
            _ => 0,
        }
    }

    /// Compare two CFIs by spine index and character offset.
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        let s1 = self.spine_index();
        let s2 = other.spine_index();
        if s1 != s2 {
            return s1.cmp(&s2);
        }
        let o1 = self.char_offset();
        let o2 = other.char_offset();
        o1.cmp(&o2)
    }

    /// Resolve IDPF element steps and assertion IDs into CSS DOM selectors and target element IDs (F1 Fix).
    pub fn resolve_dom_path(&self, html: &str) -> Option<CfiDomTarget> {
        let mut element_id = None;
        let mut selectors = Vec::new();

        for step in &self.path.steps {
            if let Some(id) = &step.element_id {
                element_id = Some(id.clone());
                selectors.push(format!("#{}", id));
            } else if !step.indirection && step.index >= 2 {
                let child_num = step.index / 2;
                selectors.push(format!("*:nth-child({})", child_num));
            }
        }

        let css_selector = if selectors.is_empty() {
            "body".to_string()
        } else {
            selectors.join(" > ")
        };

        // Check if element_id exists in target HTML
        if element_id.is_none() {
            let lower = html.to_lowercase();
            for step in &self.path.steps {
                if let Some(id) = &step.element_id {
                    if lower.contains(&format!("id=\"{}\"", id.to_lowercase()))
                        || lower.contains(&format!("id='{}'", id.to_lowercase()))
                    {
                        element_id = Some(id.clone());
                    }
                }
            }
        }

        Some(CfiDomTarget {
            element_id,
            css_selector,
            char_offset: self.char_offset(),
        })
    }

    /// Convert back to formatted `epubcfi(...)` string.
    pub fn to_cfi_string(&self) -> String {
        format!("epubcfi({})", format_path(&self.path))
    }
}

impl std::fmt::Display for Cfi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epubcfi({})", format_path(&self.path))
    }
}

fn parse_single_path(s: &str) -> Result<CfiPath, String> {
    let mut steps = Vec::new();
    let mut offset = None;

    let mut chars = s.chars().peekable();
    let mut current_num = String::new();
    let mut indirection;

    while let Some(&ch) = chars.peek() {
        if ch == '/' {
            chars.next();
            current_num.clear();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    current_num.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            if !current_num.is_empty() {
                let idx: usize = current_num.parse().unwrap_or(0);

                // Check for element_id assertion [id]
                let mut element_id = None;
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    let mut id_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        id_str.push(c);
                        chars.next();
                    }
                    element_id = Some(id_str);
                }

                // Check for indirection !
                if let Some(&'!') = chars.peek() {
                    indirection = true;
                    chars.next();
                } else {
                    indirection = false;
                }

                steps.push(CfiStep {
                    index: idx,
                    indirection,
                    element_id,
                });
            }
        } else if ch == ':' {
            chars.next();
            let mut off_num = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    off_num.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(off) = off_num.parse::<usize>() {
                offset = Some(CfiOffset::Character(off));
            }
        } else {
            chars.next();
        }
    }

    Ok(CfiPath { steps, offset })
}

fn format_path(path: &CfiPath) -> String {
    let mut out = String::new();
    for step in &path.steps {
        out.push_str(&format!("/{}", step.index));
        if let Some(ref id) = step.element_id {
            out.push_str(&format!("[{}]", id));
        }
        if step.indirection {
            out.push('!');
        }
    }
    if let Some(CfiOffset::Character(off)) = path.offset {
        out.push_str(&format!(":{}", off));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfi_parsing_and_formatting() {
        let cfi_str = "epubcfi(/6/4[chap01ref]!/4/2/10/1:42)";
        let cfi = Cfi::parse(cfi_str).unwrap();

        assert_eq!(cfi.spine_index(), 1);
        assert_eq!(cfi.char_offset(), 42);
        assert_eq!(cfi.to_string(), cfi_str);
    }
}
