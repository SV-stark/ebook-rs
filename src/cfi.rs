use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Offset in a CFI step (Character offset in text, Spatial, Temporal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CfiOffset {
    Character(usize),
    Spatial(f32, f32),
    Temporal(f32),
}

impl fmt::Display for CfiOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CfiOffset::Character(c) => write!(f, ":{}", c),
            CfiOffset::Spatial(x, y) => write!(f, "~{},{}", x, y),
            CfiOffset::Temporal(t) => write!(f, "~{}", t),
        }
    }
}

/// A single step in a CFI path (e.g., `/6/4[chap01ref]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfiStep {
    pub step: usize, // Even = Element, Odd = Text
    pub id_assertion: Option<String>,
    pub text_assertion: Option<String>,
}

impl CfiStep {
    pub fn element(step: usize) -> Self {
        Self {
            step,
            id_assertion: None,
            text_assertion: None,
        }
    }

    pub fn element_with_id(step: usize, id: &str) -> Self {
        Self {
            step,
            id_assertion: Some(id.to_string()),
            text_assertion: None,
        }
    }

    pub fn text(step: usize) -> Self {
        Self {
            step,
            id_assertion: None,
            text_assertion: None,
        }
    }
}

impl fmt::Display for CfiStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.step)?;
        if let Some(ref id) = self.id_assertion {
            write!(f, "[{}]", id)?;
        }
        if let Some(ref txt) = self.text_assertion {
            write!(f, "[{}]", txt)?;
        }
        Ok(())
    }
}

/// A path component in CFI (sequence of steps and optional offset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CfiPath {
    pub steps: Vec<CfiStep>,
    pub offset: Option<CfiOffset>,
}

impl fmt::Display for CfiPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for step in &self.steps {
            write!(f, "{}", step)?;
        }
        if let Some(ref offset) = self.offset {
            write!(f, "{}", offset)?;
        }
        Ok(())
    }
}

/// Parsed EPUB Canonical Fragment Identifier (CFI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cfi {
    pub spine_path: CfiPath,
    pub dom_path: Option<CfiPath>,
    pub range_start: Option<CfiPath>,
    pub range_end: Option<CfiPath>,
}

impl Cfi {
    /// Construct a standard CFI from spine index and optional character offset.
    pub fn from_spine_index(
        spine_index: usize,
        element_id: Option<&str>,
        char_offset: usize,
    ) -> Self {
        // Spine steps standard: /6/2 is root/spine container. Spine index 0 -> step 2, index 1 -> step 4...
        let spine_step = (spine_index + 1) * 2;
        let mut spine_steps = vec![CfiStep::element(6), CfiStep::element(spine_step)];
        if let Some(id) = element_id {
            spine_steps[1].id_assertion = Some(id.to_string());
        }

        let spine_path = CfiPath {
            steps: spine_steps,
            offset: None,
        };

        let dom_steps = vec![
            CfiStep::element(4), // body
            CfiStep::element(2), // section/div
            CfiStep::text(1),    // text node
        ];

        let dom_path = CfiPath {
            steps: dom_steps,
            offset: Some(CfiOffset::Character(char_offset)),
        };

        Self {
            spine_path,
            dom_path: Some(dom_path),
            range_start: None,
            range_end: None,
        }
    }

    /// Extract spine index from CFI.
    pub fn spine_index(&self) -> usize {
        if self.spine_path.steps.len() >= 2 {
            let step = self.spine_path.steps[1].step;
            if step >= 2 {
                return (step / 2) - 1;
            }
        }
        0
    }

    /// Extract character offset from CFI.
    pub fn char_offset(&self) -> usize {
        if let Some(ref dom) = self.dom_path {
            if let Some(CfiOffset::Character(c)) = dom.offset {
                return c;
            }
        }
        0
    }

    /// Parse CFI string format (e.g. `epubcfi(/6/4[chap01]!/4/2/1:5)`).
    pub fn parse(input: &str) -> Result<Self, String> {
        let raw = input.trim();
        let content = if raw.starts_with("epubcfi(") && raw.ends_with(')') {
            &raw[8..raw.len() - 1]
        } else {
            raw
        };

        // Check if it's a range CFI containing commas (outside of brackets)
        let parts = split_cfi_range(content);
        if parts.len() == 3 {
            // Parent path, start path, end path
            let parent_cfi = Self::parse_single_path(parts[0])?;
            let start_path = parse_cfi_path_str(parts[1])?;
            let end_path = parse_cfi_path_str(parts[2])?;

            return Ok(Self {
                spine_path: parent_cfi.spine_path,
                dom_path: parent_cfi.dom_path,
                range_start: Some(start_path),
                range_end: Some(end_path),
            });
        }

        Self::parse_single_path(content)
    }

    fn parse_single_path(content: &str) -> Result<Self, String> {
        let (spine_str, dom_str) = if let Some(idx) = content.find('!') {
            (&content[..idx], Some(&content[idx + 1..]))
        } else {
            (content, None)
        };

        let spine_path = parse_cfi_path_str(spine_str)?;
        let dom_path = if let Some(ds) = dom_str {
            Some(parse_cfi_path_str(ds)?)
        } else {
            None
        };

        Ok(Self {
            spine_path,
            dom_path,
            range_start: None,
            range_end: None,
        })
    }

    /// Compare two CFIs in document order.
    pub fn compare(&self, other: &Cfi) -> Ordering {
        // First compare spine index
        let s1 = self.spine_index();
        let s2 = other.spine_index();
        if s1 != s2 {
            return s1.cmp(&s2);
        }

        // Compare DOM steps if both present
        if let (Some(d1), Some(d2)) = (&self.dom_path, &other.dom_path) {
            let min_len = d1.steps.len().min(d2.steps.len());
            for i in 0..min_len {
                if d1.steps[i].step != d2.steps[i].step {
                    return d1.steps[i].step.cmp(&d2.steps[i].step);
                }
            }
            if d1.steps.len() != d2.steps.len() {
                return d1.steps.len().cmp(&d2.steps.len());
            }
            // Compare offsets
            let c1 = self.char_offset();
            let c2 = other.char_offset();
            return c1.cmp(&c2);
        }

        Ordering::Equal
    }
}

impl fmt::Display for Cfi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "epubcfi(")?;
        write!(f, "{}", self.spine_path)?;
        if let Some(ref dom) = self.dom_path {
            write!(f, "!{}", dom)?;
        }
        if let (Some(start), Some(end)) = (&self.range_start, &self.range_end) {
            write!(f, ",{},{}", start, end)?;
        }
        write!(f, ")")
    }
}

fn split_cfi_range(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_bracket = false;
    let mut last_idx = 0;

    for (i, ch) in input.char_indices() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            ',' if !in_bracket => {
                parts.push(&input[last_idx..i]);
                last_idx = i + 1;
            }
            _ => {}
        }
    }
    if last_idx < input.len() {
        parts.push(&input[last_idx..]);
    }
    parts
}

fn parse_cfi_path_str(input: &str) -> Result<CfiPath, String> {
    let mut steps = Vec::new();
    let mut offset = None;
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '/' {
            chars.next(); // consume '/'
            let mut num_str = String::new();
            while let Some(&digit) = chars.peek() {
                if digit.is_ascii_digit() {
                    num_str.push(digit);
                    chars.next();
                } else {
                    break;
                }
            }
            if num_str.is_empty() {
                continue;
            }
            let step_num: usize = num_str
                .parse()
                .map_err(|e| format!("Invalid step number: {}", e))?;

            let id_assertion = if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut bracket_content = String::new();
                while let Some(&b_ch) = chars.peek() {
                    chars.next();
                    if b_ch == ']' {
                        break;
                    }
                    bracket_content.push(b_ch);
                }
                Some(bracket_content)
            } else {
                None
            };
            let text_assertion = None;

            steps.push(CfiStep {
                step: step_num,
                id_assertion,
                text_assertion,
            });
        } else if ch == ':' {
            chars.next(); // consume ':'
            let mut num_str = String::new();
            while let Some(&digit) = chars.peek() {
                if digit.is_ascii_digit() {
                    num_str.push(digit);
                    chars.next();
                } else {
                    break;
                }
            }
            if !num_str.is_empty() {
                let off: usize = num_str
                    .parse()
                    .map_err(|e| format!("Invalid offset: {}", e))?;
                offset = Some(CfiOffset::Character(off));
            }
        } else {
            chars.next();
        }
    }

    Ok(CfiPath { steps, offset })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfi_parsing_and_formatting() {
        let cfi_str = "epubcfi(/6/4[chap01ref]!/4/2/10/1:5)";
        let parsed = Cfi::parse(cfi_str).unwrap();
        assert_eq!(parsed.spine_index(), 1); // step 4 -> index 1
        assert_eq!(parsed.char_offset(), 5);
        assert_eq!(parsed.to_string(), cfi_str);
    }
}
