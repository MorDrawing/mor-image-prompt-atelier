//! Prompt Decimal Classification (PDC) — Dewey-inspired shelf marks for image prompts.
//!
//! Codes nest by prefix: filter `700` includes `740`, `741`, etc.
//! Built-in schedule ships in `data/taxonomy.json`; workspace may override.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

static BUILTIN: &str = include_str!("../data/taxonomy.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Taxonomy {
    pub name: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub classes: Vec<Taxon>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Taxon {
    pub code: String,
    pub label: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub children: Vec<Taxon>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatClass {
    pub code: String,
    pub label: String,
    pub path: String,
    pub keywords: Vec<String>,
    pub depth: usize,
}

impl Taxonomy {
    pub fn load() -> Self {
        // Workspace override (editable schedule).
        let path = crate::library::data_dir().join("taxonomy.json");
        if path.is_file() {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(t) = serde_json::from_str(&raw) {
                    return t;
                }
            }
        }
        // Bundled next to repo data (when running from source with default dir).
        let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/taxonomy.json");
        if bundled.is_file() {
            if let Ok(raw) = fs::read_to_string(&bundled) {
                if let Ok(t) = serde_json::from_str(&raw) {
                    return t;
                }
            }
        }
        serde_json::from_str(BUILTIN).expect("builtin taxonomy")
    }

    /// Depth-first flat list (parents before children).
    pub fn flatten(&self) -> Vec<FlatClass> {
        let mut out = Vec::new();
        for c in &self.classes {
            flatten_into(c, &[], 0, &mut out);
        }
        out
    }

    pub fn find(&self, code: &str) -> Option<FlatClass> {
        let code = code.trim();
        self.flatten().into_iter().find(|c| c.code == code)
    }

    pub fn label_for(&self, code: &str) -> String {
        self.find(code)
            .map(|c| format!("{} · {}", c.code, c.label))
            .unwrap_or_else(|| code.to_string())
    }

    /// Top-level hundreds only (for filter chips).
    pub fn roots(&self) -> Vec<&Taxon> {
        self.classes.iter().collect()
    }

    /// Suggest best class from prompt text (keyword score).
    pub fn suggest(&self, prompt: &str) -> Option<FlatClass> {
        let text = prompt.to_lowercase();
        if text.trim().is_empty() {
            return None;
        }
        let mut best: Option<(i32, FlatClass)> = None;
        for c in self.flatten() {
            let mut score = 0i32;
            for kw in &c.keywords {
                let k = kw.to_lowercase();
                if k.is_empty() {
                    continue;
                }
                if text.contains(&k) {
                    // Longer keywords and deeper (more specific) classes win slightly.
                    score += 10 + k.len() as i32 + (c.depth as i32 * 2);
                }
            }
            // Also match label words.
            for w in c.label.to_lowercase().split_whitespace() {
                if w.len() >= 4 && text.contains(w) {
                    score += 4;
                }
            }
            if score > 0 {
                match &best {
                    None => best = Some((score, c)),
                    Some((s, _)) if score > *s => best = Some((score, c)),
                    Some((s, prev)) if score == *s && c.depth > prev.depth => {
                        best = Some((score, c));
                    }
                    _ => {}
                }
            }
        }
        best.map(|(_, c)| c)
    }
}

fn flatten_into(t: &Taxon, ancestors: &[String], depth: usize, out: &mut Vec<FlatClass>) {
    let mut path_parts = ancestors.to_vec();
    path_parts.push(t.label.clone());
    out.push(FlatClass {
        code: t.code.clone(),
        label: t.label.clone(),
        path: path_parts.join(" › "),
        keywords: t.keywords.clone(),
        depth,
    });
    for child in &t.children {
        flatten_into(child, &path_parts, depth + 1, out);
    }
}

/// True if `code` sits under filter (Dewey-style).
/// `700` → 700–799 · `740` → 740–749 · `741` → 741…
pub fn code_matches_filter(code: &str, filter: &str) -> bool {
    let code = code.trim();
    let filter = filter.trim();
    if filter.is_empty() || filter == "all" {
        return true;
    }
    if code.is_empty() {
        return false;
    }
    if code == filter || code.starts_with(filter) {
        return true;
    }
    // Hundreds shelf: 700 matches any 7xx
    if filter.len() >= 3 && filter.ends_with("00") {
        return code.starts_with(&filter[..1]);
    }
    // Tens shelf: 740 matches 74x
    if filter.len() >= 3 && filter.ends_with('0') {
        return code.starts_with(&filter[..2]);
    }
    false
}

/// Tag prefix used when persisting class into mflash card tags.
pub const CLASS_TAG_PREFIX: &str = "pdc:";

pub fn class_from_tags(tags: &[String]) -> Option<String> {
    tags.iter().find_map(|t| {
        let t = t.trim();
        t.strip_prefix(CLASS_TAG_PREFIX)
            .or_else(|| t.strip_prefix("class:"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

pub fn tags_with_class(tags: &[String], class_code: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .filter(|t| {
            let t = t.trim();
            !t.starts_with(CLASS_TAG_PREFIX) && !t.starts_with("class:")
        })
        .cloned()
        .collect();
    if let Some(c) = class_code.map(str::trim).filter(|s| !s.is_empty()) {
        out.push(format!("{CLASS_TAG_PREFIX}{c}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_suggests_mucha() {
        let tax = Taxonomy::load();
        assert!(tax.classes.len() >= 8);
        let s = tax.suggest("a grey pitbull in the style of Alphonse Mucha, art nouveau poster");
        let code = s.expect("suggest").code;
        assert!(
            code.starts_with("7") || code.starts_with("2"),
            "expected arts or animal, got {code}"
        );
    }

    #[test]
    fn prefix_filter() {
        assert!(code_matches_filter("740", "700"));
        assert!(code_matches_filter("700", "700"));
        assert!(!code_matches_filter("100", "700"));
        assert!(code_matches_filter("740", "all"));
    }
}
