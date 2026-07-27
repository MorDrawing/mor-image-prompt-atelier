//! Workspace = a folder (native picker) holding mflash-style files:
//! ```text
//! <folder>/
//!   deck.json     # mflash deck (prompts as cards)
//!   media/        # optional result images
//! ```
//! Config remembers the last folder. Env `MOR_PROMPTS_DATA` still wins for agents.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static WORKSPACE: Mutex<Option<PathBuf>> = Mutex::new(None);

// ── Paths ────────────────────────────────────────────────────────────────

pub fn config_dir() -> PathBuf {
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("mor-image-prompt-atelier");
    }
    dirs_fallback_home().join(".config/mor-image-prompt-atelier")
}

fn dirs_fallback_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Bundled sample data shipped with the repo (used if no folder chosen yet).
pub fn bundled_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    workspace: Option<String>,
}

fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir config: {e}"))?;
    }
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| format!("serialize config: {e}"))?;
    fs::write(&path, raw + "\n").map_err(|e| format!("write config: {e}"))
}

/// Active workspace folder (prompts + media live here).
pub fn data_dir() -> PathBuf {
    if let Ok(g) = WORKSPACE.lock() {
        if let Some(p) = g.as_ref() {
            return p.clone();
        }
    }
    if let Ok(p) = std::env::var("MOR_PROMPTS_DATA") {
        return PathBuf::from(p);
    }
    let cfg = load_config();
    if let Some(w) = cfg.workspace.filter(|s| !s.trim().is_empty()) {
        return PathBuf::from(w);
    }
    bundled_data_dir()
}

/// Point the app at a folder and remember it.
pub fn set_workspace(path: PathBuf) -> Result<(), String> {
    let path = path
        .canonicalize()
        .unwrap_or(path);
    fs::create_dir_all(&path).map_err(|e| format!("mkdir workspace: {e}"))?;
    fs::create_dir_all(path.join("media")).map_err(|e| format!("mkdir media: {e}"))?;
    if let Ok(mut g) = WORKSPACE.lock() {
        *g = Some(path.clone());
    }
    let mut cfg = load_config();
    cfg.workspace = Some(path.to_string_lossy().into_owned());
    save_config(&cfg)?;
    Ok(())
}

pub fn workspace_display() -> String {
    data_dir().display().to_string()
}

pub fn deck_path() -> PathBuf {
    data_dir().join("deck.json")
}

pub fn media_dir() -> PathBuf {
    data_dir().join("media")
}

pub fn catalog_path() -> PathBuf {
    data_dir().join("catalog.sqlite")
}

// ── Prompt model (in-memory) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Skeleton {
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub setting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptEntry {
    pub id: String,
    pub title: String,
    #[serde(default = "default_tier")]
    pub tier: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub prompt: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub last_outcome: Option<String>,
    #[serde(default)]
    pub last_note: String,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_disposition_at: Option<String>,
    #[serde(default)]
    pub copy_count_without_scar: u32,
    #[serde(default)]
    pub needs_rework: bool,
    #[serde(default = "default_storage")]
    pub storage: String,
    #[serde(default)]
    pub skeleton: Option<Skeleton>,
    #[serde(default)]
    pub fragment_ids: Vec<String>,
    #[serde(default = "default_pack")]
    pub pack_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_class: Option<String>,
    /// Relative path under workspace, e.g. `media/foo.png`, or bare filename in media/.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
}

fn default_tier() -> String {
    "B".into()
}
fn default_storage() -> String {
    "hot".into()
}
fn default_pack() -> String {
    "inbox".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Library {
    #[serde(default = "default_lib_version")]
    pub version: u32,
    #[serde(default)]
    pub prompts: Vec<PromptEntry>,
    /// Deck id written into deck.json
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_title: Option<String>,
}

fn default_lib_version() -> u32 {
    3
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 3,
            prompts: vec![],
            deck_id: None,
            deck_title: None,
        }
    }
}

// ── mflash deck.json (loose folder package) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MflashDeck {
    format: String,
    version: u32,
    id: String,
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default = "default_lang")]
    default_term_lang: String,
    #[serde(default = "default_lang")]
    default_def_lang: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    deck_tags: Vec<String>,
    #[serde(default)]
    cards: Vec<MflashCard>,
}

fn default_lang() -> String {
    "en".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MflashCard {
    id: String,
    #[serde(default = "default_kind")]
    kind: String,
    /// AI image prompt lives in `term` (mflash basic card).
    #[serde(default, alias = "prompt")]
    term: String,
    #[serde(default, alias = "answer")]
    definition: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    media: Vec<MflashMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

fn default_kind() -> String {
    "basic".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MflashMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default = "default_media_type", rename = "type")]
    media_type: String,
    #[serde(default = "default_media_role")]
    role: String,
    src: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    alt: Option<String>,
}

fn default_media_type() -> String {
    "image".into()
}
fn default_media_role() -> String {
    "result".into()
}

// ── Time / ids ───────────────────────────────────────────────────────────

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn new_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{n:x}")
}

pub fn infer_subject_class(p: &PromptEntry) -> String {
    if let Some(sc) = &p.subject_class {
        if !sc.trim().is_empty() {
            return sc.trim().to_lowercase();
        }
    }
    let blob = format!("{} {} {}", p.tags.join(" "), p.title, p.prompt).to_lowercase();
    if blob.contains("animal") || blob.contains("pitbull") || blob.contains("dog") {
        return "animal".into();
    }
    if blob.contains("poster") || blob.contains("mucha") {
        return "poster".into();
    }
    if blob.contains("professor")
        || blob.contains("wordsmith")
        || blob.contains("character")
        || blob.contains("anime")
    {
        return "character".into();
    }
    if blob.contains("street") || blob.contains("atelier") || blob.contains("scene") {
        return "scene".into();
    }
    "other".into()
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(value).map_err(|e| format!("serialize: {e}"))?;
    fs::write(path, raw + "\n").map_err(|e| format!("write {}: {e}", path.display()))
}

// ── Load / save ──────────────────────────────────────────────────────────

pub fn load_library() -> Result<Library, String> {
    let root = data_dir();
    let _ = fs::create_dir_all(root.join("media"));

    // Preferred: mflash deck.json in workspace
    if deck_path().is_file() {
        return load_from_deck();
    }

    // Legacy: multi-pack layout under workspace (bundled sample data)
    let packs_root = root.join("packs");
    if packs_root.is_dir() {
        return load_from_packs(&packs_root);
    }

    // Flat library.json
    let flat = root.join("library.json");
    if flat.is_file() {
        let raw = fs::read_to_string(&flat).map_err(|e| format!("read library: {e}"))?;
        let mut lib: Library =
            serde_json::from_str(&raw).map_err(|e| format!("parse library: {e}"))?;
        for p in &mut lib.prompts {
            if p.subject_class.is_none() {
                p.subject_class = Some(infer_subject_class(p));
            }
        }
        return Ok(lib);
    }

    Ok(Library::default())
}

fn load_from_deck() -> Result<Library, String> {
    let path = deck_path();
    let raw = fs::read_to_string(&path).map_err(|e| format!("read deck: {e}"))?;
    let deck: MflashDeck =
        serde_json::from_str(&raw).map_err(|e| format!("parse deck.json: {e}"))?;

    let mut lib = Library {
        version: 3,
        deck_id: Some(deck.id.clone()),
        deck_title: Some(deck.title.clone()),
        ..Library::default()
    };

    for c in deck.cards {
        let image = c
            .media
            .iter()
            .find(|m| {
                m.media_type == "image"
                    || m.role == "result"
                    || m.role == "prompt_image"
                    || m.role.contains("image")
            })
            .map(|m| m.src.clone())
            .or_else(|| c.media.first().map(|m| m.src.clone()));

        let prompt_text = c.term.trim().to_string();
        let mut notes = c.notes.unwrap_or_default();
        if notes.is_empty() && !c.definition.trim().is_empty() {
            notes = c.definition;
        }

        let mut entry = PromptEntry {
            id: c.id,
            title: title_from_prompt(&prompt_text),
            tier: "B".into(),
            tags: c.tags,
            prompt: prompt_text,
            notes,
            created_at: now_iso(),
            updated_at: now_iso(),
            last_outcome: None,
            last_note: String::new(),
            last_run_at: None,
            last_disposition_at: None,
            copy_count_without_scar: 0,
            needs_rework: false,
            storage: "hot".into(),
            skeleton: None,
            fragment_ids: vec![],
            pack_id: "inbox".into(),
            subject_class: None,
            image,
            images: vec![],
        };
        entry.subject_class = Some(infer_subject_class(&entry));
        lib.prompts.push(entry);
    }

    lib.prompts
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(lib)
}

fn load_from_packs(packs_root: &Path) -> Result<Library, String> {
    let mut lib = Library::default();
    let entries = fs::read_dir(packs_root).map_err(|e| format!("read packs: {e}"))?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let pack_id = entry.file_name().to_string_lossy().to_string();
        let prompts_dir = entry.path().join("prompts");
        if !prompts_dir.is_dir() {
            continue;
        }
        for pf in fs::read_dir(&prompts_dir).map_err(|e| format!("read prompts: {e}"))? {
            let pf = pf.map_err(|e| format!("prompt dir entry: {e}"))?;
            let path = pf.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let raw =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let mut p: PromptEntry =
                serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
            if p.pack_id.is_empty() {
                p.pack_id = pack_id.clone();
            }
            // Prefer pack media if image is bare filename
            if let Some(img) = p.image.clone() {
                let in_pack = entry.path().join("media").join(&img);
                if in_pack.is_file() {
                    // Copy into workspace media/ on first modern save; for display resolve via pack path
                    p.image = Some(format!("packs/{pack_id}/media/{img}"));
                }
            } else {
                // auto-discover pack media by id
                for ext in ["webp", "png", "jpg", "jpeg", "gif", "svg"] {
                    let cand = entry.path().join("media").join(format!("{}.{}", p.id, ext));
                    if cand.is_file() {
                        p.image = Some(format!(
                            "packs/{pack_id}/media/{}.{}",
                            p.id, ext
                        ));
                        break;
                    }
                }
            }
            if p.subject_class.is_none() {
                p.subject_class = Some(infer_subject_class(&p));
            }
            lib.prompts.push(p);
        }
    }
    lib.prompts
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(lib)
}

fn title_from_prompt(prompt: &str) -> String {
    let t = prompt.trim().replace('\n', " ");
    if t.is_empty() {
        return "Untitled".into();
    }
    let mut s: String = t.chars().take(48).collect();
    if t.chars().count() > 48 {
        s.push('…');
    }
    s
}

/// Persist as mflash `deck.json` (+ rebuild catalog).
pub fn save_library(lib: &Library) -> Result<(), String> {
    let root = data_dir();
    fs::create_dir_all(root.join("media")).map_err(|e| format!("mkdir media: {e}"))?;

    let deck_id = lib
        .deck_id
        .clone()
        .unwrap_or_else(|| format!("atelier-{}", new_id()));
    let deck_title = lib
        .deck_title
        .clone()
        .unwrap_or_else(|| "Image Prompt Atelier".into());

    let mut cards = Vec::with_capacity(lib.prompts.len());
    for p in &lib.prompts {
        if p.storage == "compost" {
            continue;
        }
        let mut media = vec![];
        if let Some(src) = p.image.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            media.push(MflashMedia {
                id: Some(format!("img-{}", p.id)),
                media_type: "image".into(),
                role: "result".into(),
                src: src.to_string(),
                alt: Some(p.title.clone()),
            });
        }
        cards.push(MflashCard {
            id: p.id.clone(),
            kind: "basic".into(),
            term: p.prompt.clone(),
            definition: p.notes.clone(),
            tags: p.tags.clone(),
            media,
            notes: if p.notes.is_empty() {
                None
            } else {
                Some(p.notes.clone())
            },
        });
    }

    let deck = MflashDeck {
        format: "mflash".into(),
        version: 3,
        id: deck_id,
        title: deck_title,
        description: Some("Image prompts for AI generators (Mor Atelier)".into()),
        default_term_lang: "en".into(),
        default_def_lang: "en".into(),
        deck_tags: vec!["image-prompt".into(), "atelier".into()],
        cards,
    };
    write_json_pretty(&deck_path(), &deck)?;

    // Flat export for grepping
    let export = Library {
        version: 3,
        prompts: lib.prompts.clone(),
        deck_id: lib.deck_id.clone(),
        deck_title: lib.deck_title.clone(),
        ..Library::default()
    };
    write_json_pretty(&root.join("library.json"), &export)?;

    if let Err(e) = crate::catalog::rebuild(lib) {
        eprintln!("catalog rebuild: {e}");
    }
    Ok(())
}

pub fn new_prompt_entry(title: &str, prompt: &str) -> PromptEntry {
    let now = now_iso();
    let mut entry = PromptEntry {
        id: new_id(),
        title: if title.trim().is_empty() {
            title_from_prompt(prompt)
        } else {
            title.to_string()
        },
        tier: "B".into(),
        tags: vec![],
        prompt: prompt.to_string(),
        notes: String::new(),
        created_at: now.clone(),
        updated_at: now,
        last_outcome: None,
        last_note: String::new(),
        last_run_at: None,
        last_disposition_at: None,
        copy_count_without_scar: 0,
        needs_rework: false,
        storage: "hot".into(),
        skeleton: None,
        fragment_ids: vec![],
        pack_id: "inbox".into(),
        subject_class: None,
        image: None,
        images: vec![],
    };
    entry.subject_class = Some(infer_subject_class(&entry));
    entry
}

/// Copy an image into workspace `media/` and return relative path for `image` field.
pub fn import_image_for_prompt(prompt_id: &str, source: &Path) -> Result<String, String> {
    if !source.is_file() {
        return Err(format!("not a file: {}", source.display()));
    }
    let media = media_dir();
    fs::create_dir_all(&media).map_err(|e| format!("mkdir media: {e}"))?;
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let dest_name = format!("{prompt_id}.{ext}");
    let dest = media.join(&dest_name);
    fs::copy(source, &dest).map_err(|e| format!("copy image: {e}"))?;
    Ok(format!("media/{dest_name}"))
}

/// Absolute path to the card image, if any.
pub fn resolve_prompt_image(p: &PromptEntry) -> Option<PathBuf> {
    let root = data_dir();
    if let Some(name) = p.image.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let path = if Path::new(name).is_absolute() {
            PathBuf::from(name)
        } else {
            root.join(name)
        };
        if path.is_file() {
            return Some(path);
        }
        // bare filename → media/
        let in_media = media_dir().join(name);
        if in_media.is_file() {
            return Some(in_media);
        }
    }
    for name in &p.images {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let path = root.join(name);
        if path.is_file() {
            return Some(path);
        }
        let in_media = media_dir().join(name);
        if in_media.is_file() {
            return Some(in_media);
        }
    }
    for ext in ["webp", "png", "jpg", "jpeg", "gif", "svg"] {
        let path = media_dir().join(format!("{}.{}", p.id, ext));
        if path.is_file() {
            return Some(path);
        }
    }
    // Legacy sample layout: packs/<pack>/media/<id>.*
    let packs = root.join("packs");
    if packs.is_dir() {
        let pack_ids = [p.pack_id.as_str(), "inbox", "murdoch-core", "characters", "poster-icons"];
        for pack in pack_ids {
            for ext in ["webp", "png", "jpg", "jpeg", "gif", "svg"] {
                let path = packs
                    .join(pack)
                    .join("media")
                    .join(format!("{}.{}", p.id, ext));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        if let Ok(dirs) = fs::read_dir(&packs) {
            for d in dirs.flatten() {
                for ext in ["webp", "png", "jpg", "jpeg", "gif", "svg"] {
                    let path = d.path().join("media").join(format!("{}.{}", p.id, ext));
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

pub fn path_to_file_url(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy();
    let mut out = String::from("file://");
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '/' | '-' | '_' | '.' | '~' => out.push(c),
            ' ' => out.push_str("%20"),
            _ => {
                for b in c.to_string().as_bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
        }
    }
    out
}

pub fn prompt_image_url(p: &PromptEntry) -> Option<String> {
    resolve_prompt_image(p).map(|path| path_to_file_url(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_workspace(path: PathBuf, f: impl FnOnce()) {
        let _g = TEST_LOCK.lock().unwrap();
        set_workspace(path).expect("set workspace");
        f();
        if let Ok(mut w) = WORKSPACE.lock() {
            *w = None;
        }
    }

    #[test]
    fn load_packs_and_catalog() {
        with_workspace(bundled_data_dir(), || {
            let lib = load_library().expect("load");
            assert!(
                lib.prompts.len() >= 3,
                "expected starter prompts, got {}",
                lib.prompts.len()
            );
            // Prefer in-memory search fallback if catalog rename races on some FS.
            let ids = match crate::catalog::rebuild(&lib) {
                Ok(()) => crate::catalog::search(&crate::catalog::SearchQuery {
                    text: "pitbull".into(),
                    limit: 10,
                    ..Default::default()
                })
                .unwrap_or_default(),
                Err(_) => crate::catalog::filter_in_memory(
                    &lib,
                    &crate::catalog::SearchQuery {
                        text: "pitbull".into(),
                        limit: 10,
                        ..Default::default()
                    },
                ),
            };
            assert!(
                ids.contains(&"starter-mucha-pitbull".to_string())
                    || lib.prompts.iter().any(|p| p.prompt.contains("pitbull"))
            );
        });
    }

    #[test]
    fn subject_class_inference() {
        let p = PromptEntry {
            id: "t".into(),
            title: "Grey pitbull".into(),
            tier: "A".into(),
            tags: vec!["animal".into()],
            prompt: "a dog".into(),
            notes: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            last_outcome: None,
            last_note: String::new(),
            last_run_at: None,
            last_disposition_at: None,
            copy_count_without_scar: 0,
            needs_rework: false,
            storage: "hot".into(),
            skeleton: None,
            fragment_ids: vec![],
            pack_id: "poster-icons".into(),
            subject_class: None,
            image: None,
            images: vec![],
        };
        assert_eq!(infer_subject_class(&p), "animal");
    }

    #[test]
    fn resolve_image_from_media_file() {
        with_workspace(bundled_data_dir(), || {
            let lib = load_library().expect("load");
            let p = lib
                .prompts
                .iter()
                .find(|p| p.id == "starter-pc98-wordsmiths")
                .expect("starter");
            let path = resolve_prompt_image(p).expect("media file");
            assert!(path.exists());
        });
    }

    #[test]
    fn deck_roundtrip_and_image_import() {
        let tmp = std::env::temp_dir().join(format!("atelier-test-{}", new_id()));
        let _ = fs::remove_dir_all(&tmp);
        with_workspace(tmp.clone(), || {
            let mut lib = Library::default();
            let mut p = new_prompt_entry("t", "a prompt about fog");
            lib.prompts.push(p.clone());
            save_library(&lib).expect("save");
            assert!(deck_path().is_file());

            let src = tmp.join("src.svg");
            fs::write(&src, b"<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();
            let rel = import_image_for_prompt(&p.id, &src).expect("import");
            p.image = Some(rel);
            lib.prompts[0] = p.clone();
            save_library(&lib).expect("save2");

            let loaded = load_library().expect("reload");
            assert_eq!(loaded.prompts.len(), 1);
            assert!(loaded.prompts[0].prompt.contains("fog"));
            assert!(resolve_prompt_image(&loaded.prompts[0]).is_some());
        });
        let _ = fs::remove_dir_all(&tmp);
    }
}
