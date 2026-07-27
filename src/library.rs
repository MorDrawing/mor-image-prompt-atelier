//! Local-first library: decks of image+prompt cards + optional desk + flat export.
//!
//! Layout (UI says "decks"; on disk still `packs/`):
//! ```text
//! data/
//!   packs/<deck-id>/pack.json
//!   packs/<deck-id>/prompts/<card-id>.json
//!   packs/<deck-id>/media/          # card faces / reference gens
//!   desk.json                       # optional missions (not primary UI)
//!   library.json                    # flat export for compatibility
//!   catalog.sqlite                  # rebuildable FTS index
//!   styles.json / flora.json
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("MOR_PROMPTS_DATA") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

pub fn library_path() -> PathBuf {
    data_dir().join("library.json")
}

pub fn desk_path() -> PathBuf {
    data_dir().join("desk.json")
}

pub fn packs_dir() -> PathBuf {
    data_dir().join("packs")
}

pub fn styles_path() -> PathBuf {
    data_dir().join("styles.json")
}

pub fn flora_path() -> PathBuf {
    data_dir().join("flora.json")
}

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
    /// "hot" | "cold" | "compost"
    #[serde(default = "default_storage")]
    pub storage: String,
    #[serde(default)]
    pub skeleton: Option<Skeleton>,
    #[serde(default)]
    pub fragment_ids: Vec<String>,
    /// Pack slug this prompt lives under (e.g. murdoch-core). UI calls these decks.
    #[serde(default = "default_pack")]
    pub pack_id: String,
    /// Facet: character | animal | scene | poster | abstract | other
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_class: Option<String>,
    /// Filename under `packs/<pack_id>/media/` (e.g. `starter.png`). Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Extra reference filenames under the same media/ folder.
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
pub struct NextExperiment {
    pub prompt_id: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub note: String,
    #[serde(default = "default_open")]
    pub status: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_open() -> String {
    "open".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExperimentHistoryItem {
    pub prompt_id: String,
    pub action: String,
    #[serde(default)]
    pub note: String,
    pub status: String,
    pub closed_at: String,
}

/// Desk state lives beside packs (like mflash .progress).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Desk {
    #[serde(default = "default_desk_version")]
    pub version: u32,
    #[serde(default)]
    pub next_experiment: Option<NextExperiment>,
    #[serde(default)]
    pub experiment_history: Vec<ExperimentHistoryItem>,
}

fn default_desk_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackMeta {
    #[serde(default = "default_pack_format")]
    pub format: String,
    #[serde(default = "default_pack_version")]
    pub version: u32,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

fn default_pack_format() -> String {
    "mor-prompt-pack".into()
}

fn default_pack_version() -> u32 {
    1
}

impl PackMeta {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let id = id.into();
        let title = title.into();
        Self {
            format: default_pack_format(),
            version: 1,
            id,
            title,
            description: None,
            tags: vec![],
            license: Some("UNLICENSE".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Library {
    #[serde(default = "default_lib_version")]
    pub version: u32,
    #[serde(default)]
    pub next_experiment: Option<NextExperiment>,
    #[serde(default)]
    pub experiment_history: Vec<ExperimentHistoryItem>,
    #[serde(default)]
    pub prompts: Vec<PromptEntry>,
    /// Pack metadata indexed by pack id (not always in library.json).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packs: HashMap<String, PackMeta>,
}

fn default_lib_version() -> u32 {
    3
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 3,
            next_experiment: None,
            experiment_history: vec![],
            prompts: vec![],
            packs: HashMap::new(),
        }
    }
}

pub fn rework_count(lib: &Library) -> usize {
    lib.prompts.iter().filter(|p| p.needs_rework).count()
}

/// Classify a prompt for faceted search.
pub fn infer_subject_class(p: &PromptEntry) -> String {
    if let Some(sc) = &p.subject_class {
        if !sc.trim().is_empty() {
            return sc.trim().to_lowercase();
        }
    }
    let blob = format!(
        "{} {} {}",
        p.tags.join(" "),
        p.title,
        p.skeleton
            .as_ref()
            .map(|s| s.subject.as_str())
            .unwrap_or("")
    )
    .to_lowercase();
    if blob.contains("animal")
        || blob.contains("pitbull")
        || blob.contains("dog")
        || blob.contains("cat")
        || blob.contains("bird")
    {
        return "animal".into();
    }
    if blob.contains("poster") || blob.contains("mucha") || blob.contains("banner") {
        return "poster".into();
    }
    if blob.contains("professor")
        || blob.contains("wordsmith")
        || blob.contains("scholar")
        || blob.contains("character")
        || blob.contains("anime")
        || blob.contains("person")
    {
        return "character".into();
    }
    if blob.contains("street")
        || blob.contains("atelier")
        || blob.contains("landscape")
        || blob.contains("interior")
        || blob.contains("scene")
    {
        return "scene".into();
    }
    if p.skeleton
        .as_ref()
        .map(|s| !s.subject.trim().is_empty())
        .unwrap_or(false)
    {
        return "character".into();
    }
    "other".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StylePack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StylesFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub styles: Vec<StylePack>,
    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub lighting: Vec<String>,
    #[serde(default)]
    pub composition: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FloraFragment {
    pub id: String,
    pub text: String,
    pub slot: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub style_affinity: Vec<String>,
    #[serde(default = "default_pool")]
    pub pool: String,
    #[serde(default)]
    pub weight: i32,
}

fn default_pool() -> String {
    "experimental".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FloraFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub fragments: Vec<FloraFragment>,
}

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

pub fn sanitize_pack_id(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "inbox".into()
    } else {
        s
    }
}

fn pack_dir(pack_id: &str) -> PathBuf {
    packs_dir().join(sanitize_pack_id(pack_id))
}

fn prompt_file(pack_id: &str, prompt_id: &str) -> PathBuf {
    pack_dir(pack_id).join("prompts").join(format!("{prompt_id}.json"))
}

fn pack_meta_path(pack_id: &str) -> PathBuf {
    pack_dir(pack_id).join("pack.json")
}

pub fn load_desk() -> Result<Desk, String> {
    let path = desk_path();
    if !path.exists() {
        return Ok(Desk::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read desk: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse desk: {e}"))
}

pub fn save_desk(desk: &Desk) -> Result<(), String> {
    let path = desk_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir data: {e}"))?;
    }
    let raw = serde_json::to_string_pretty(desk).map_err(|e| format!("serialize desk: {e}"))?;
    fs::write(&path, raw + "\n").map_err(|e| format!("write desk: {e}"))
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(value).map_err(|e| format!("serialize: {e}"))?;
    fs::write(path, raw + "\n").map_err(|e| format!("write {}: {e}", path.display()))
}

pub fn ensure_pack_meta(pack_id: &str, title_hint: Option<&str>) -> Result<PackMeta, String> {
    let id = sanitize_pack_id(pack_id);
    let path = pack_meta_path(&id);
    if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| format!("read pack: {e}"))?;
        return serde_json::from_str(&raw).map_err(|e| format!("parse pack: {e}"));
    }
    let title = title_hint
        .map(|s| s.to_string())
        .unwrap_or_else(|| title_case_slug(&id));
    let mut meta = PackMeta::new(&id, title);
    meta.description = Some(format!("Prompt pack: {id}"));
    write_json_pretty(&path, &meta)?;
    // media dir placeholder
    let media = pack_dir(&id).join("media");
    let _ = fs::create_dir_all(&media);
    Ok(meta)
}

fn title_case_slug(slug: &str) -> String {
    slug.split(|c| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Load library from packs/ (+ desk). Migrates legacy library.json if needed.
pub fn load_library() -> Result<Library, String> {
    let packs_root = packs_dir();
    let has_packs = packs_root.is_dir()
        && fs::read_dir(&packs_root)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

    if !has_packs {
        // Legacy single-file library: migrate once.
        if library_path().exists() {
            let mut lib = load_library_flat()?;
            migrate_to_packs(&mut lib)?;
            return Ok(lib);
        }
        return Ok(Library::default());
    }

    let mut lib = Library {
        version: 3,
        next_experiment: None,
        experiment_history: vec![],
        prompts: vec![],
        packs: HashMap::new(),
    };

    let desk = load_desk().unwrap_or_default();
    lib.next_experiment = desk.next_experiment;
    lib.experiment_history = desk.experiment_history;

    let entries = fs::read_dir(&packs_root).map_err(|e| format!("read packs: {e}"))?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let pack_id = entry.file_name().to_string_lossy().to_string();
        let meta = ensure_pack_meta(&pack_id, None)?;
        lib.packs.insert(pack_id.clone(), meta);

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
            let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let mut p: PromptEntry =
                serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
            if p.pack_id.is_empty() || p.pack_id == "inbox" {
                // Prefer directory name as pack.
                p.pack_id = pack_id.clone();
            } else {
                p.pack_id = sanitize_pack_id(&p.pack_id);
            }
            if p.subject_class.is_none() {
                p.subject_class = Some(infer_subject_class(&p));
            }
            lib.prompts.push(p);
        }
    }

    // Prefer desk; if empty desk and flat library still has mission, import once.
    if lib.next_experiment.is_none() && library_path().exists() {
        if let Ok(flat) = load_library_flat() {
            if lib.next_experiment.is_none() {
                lib.next_experiment = flat.next_experiment;
            }
            if lib.experiment_history.is_empty() {
                lib.experiment_history = flat.experiment_history;
            }
        }
    }

    lib.prompts
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(lib)
}

fn load_library_flat() -> Result<Library, String> {
    let path = library_path();
    if !path.exists() {
        return Ok(Library::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read library: {e}"))?;
    let mut lib: Library =
        serde_json::from_str(&raw).map_err(|e| format!("parse library: {e}"))?;
    for p in &mut lib.prompts {
        if p.pack_id.is_empty() {
            p.pack_id = assign_pack_for_legacy(p);
        }
        if p.subject_class.is_none() {
            p.subject_class = Some(infer_subject_class(p));
        }
    }
    Ok(lib)
}

fn assign_pack_for_legacy(p: &PromptEntry) -> String {
    let tags: HashSet<_> = p.tags.iter().map(|t| t.to_lowercase()).collect();
    if tags.iter().any(|t| t.contains("pc98") || t.contains("murdoch")) {
        return "murdoch-core".into();
    }
    if tags.iter().any(|t| t.contains("mucha") || t.contains("art-nouveau")) {
        return "poster-icons".into();
    }
    if tags
        .iter()
        .any(|t| t.contains("professor") || t.contains("noir") || t.contains("scholar"))
    {
        return "characters".into();
    }
    if tags.iter().any(|t| t.contains("animal") || t.contains("pitbull")) {
        return "poster-icons".into();
    }
    "inbox".into()
}

/// One-time (idempotent) write of in-memory library into packs + desk.
pub fn migrate_to_packs(lib: &mut Library) -> Result<(), String> {
    for p in &mut lib.prompts {
        if p.pack_id.is_empty() {
            p.pack_id = assign_pack_for_legacy(p);
        }
        p.pack_id = sanitize_pack_id(&p.pack_id);
        if p.subject_class.is_none() {
            p.subject_class = Some(infer_subject_class(p));
        }
    }
    // Ensure pack metas with friendly titles
    let titles: HashMap<&str, &str> = [
        ("murdoch-core", "Murdoch Core"),
        ("characters", "Characters & Archetypes"),
        ("poster-icons", "Poster Icons"),
        ("inbox", "Inbox"),
    ]
    .into_iter()
    .collect();
    for p in &lib.prompts {
        let hint = titles.get(p.pack_id.as_str()).copied();
        let meta = ensure_pack_meta(&p.pack_id, hint)?;
        lib.packs.insert(p.pack_id.clone(), meta);
    }
    save_library(lib)?;
    Ok(())
}

/// Persist: packs/*.json + desk.json + flat library.json export + rebuild catalog.
pub fn save_library(lib: &Library) -> Result<(), String> {
    let mut lib = lib.clone();
    lib.version = 3;

    // Desk sidecar
    let desk = Desk {
        version: 1,
        next_experiment: lib.next_experiment.clone(),
        experiment_history: lib.experiment_history.clone(),
    };
    save_desk(&desk)?;

    // Normalize packs on entries
    for p in &mut lib.prompts {
        p.pack_id = sanitize_pack_id(&p.pack_id);
        if p.subject_class.is_none() {
            p.subject_class = Some(infer_subject_class(p));
        }
    }

    // Track written paths for GC
    let mut keep: HashSet<PathBuf> = HashSet::new();
    let mut pack_ids: HashSet<String> = HashSet::new();

    for p in &lib.prompts {
        pack_ids.insert(p.pack_id.clone());
        if !lib.packs.contains_key(&p.pack_id) {
            let meta = ensure_pack_meta(&p.pack_id, None)?;
            lib.packs.insert(p.pack_id.clone(), meta);
        } else {
            ensure_pack_meta(&p.pack_id, None)?;
        }
        let path = prompt_file(&p.pack_id, &p.id);
        write_json_pretty(&path, p)?;
        keep.insert(path);
    }

    // GC deleted prompt files under packs we touch
    if packs_dir().is_dir() {
        if let Ok(dirs) = fs::read_dir(packs_dir()) {
            for d in dirs.flatten() {
                let prompts_dir = d.path().join("prompts");
                if !prompts_dir.is_dir() {
                    continue;
                }
                if let Ok(files) = fs::read_dir(&prompts_dir) {
                    for f in files.flatten() {
                        let path = f.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json")
                            && !keep.contains(&path)
                        {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    // Flat export (compatible with older tools / git grepping)
    let export = Library {
        version: 3,
        next_experiment: lib.next_experiment.clone(),
        experiment_history: lib.experiment_history.clone(),
        prompts: lib.prompts.clone(),
        packs: HashMap::new(), // keep export lean
    };
    write_json_pretty(&library_path(), &export)?;

    // Catalog
    if let Err(e) = crate::catalog::rebuild(&lib) {
        // Non-fatal for IO path; surface as soft error
        eprintln!("catalog rebuild: {e}");
    }

    Ok(())
}

pub fn load_styles() -> Result<StylesFile, String> {
    let path = styles_path();
    if !path.exists() {
        return Ok(StylesFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read styles: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse styles: {e}"))
}

pub fn load_flora() -> Result<FloraFile, String> {
    let path = flora_path();
    if !path.exists() {
        return Ok(FloraFile::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read flora: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse flora: {e}"))
}

pub fn save_flora(flora: &FloraFile) -> Result<(), String> {
    write_json_pretty(&flora_path(), flora)
}

/// Near-duplicate detection: Jaccard of clause tokens.
pub fn find_cousins(lib: &Library, prompt: &str, exclude_id: Option<&str>) -> Vec<(String, String, f32)> {
    let needle = clause_set(prompt);
    if needle.is_empty() {
        return vec![];
    }
    let mut hits = vec![];
    for p in &lib.prompts {
        if exclude_id.is_some_and(|id| id == p.id) {
            continue;
        }
        let other = clause_set(&p.prompt);
        let sim = jaccard(&needle, &other);
        if sim >= 0.55 {
            hits.push((p.id.clone(), p.title.clone(), sim));
        }
    }
    hits.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    hits
}

fn clause_set(prompt: &str) -> HashSet<String> {
    prompt
        .split(|c: char| c == ',' || c == ';')
        .map(|s| {
            s.trim()
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty() && s.len() > 3)
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

pub fn bump_flora_for_prompt(flora: &mut FloraFile, fragment_ids: &[String], delta: i32) {
    for id in fragment_ids {
        if let Some(f) = flora.fragments.iter_mut().find(|x| &x.id == id) {
            f.weight = (f.weight + delta).clamp(1, 50);
        }
    }
}

pub fn pick_flora(
    flora: &FloraFile,
    style_id: Option<&str>,
    slot: &str,
    pool: Option<&str>,
    max: usize,
    already: &[String],
) -> Vec<FloraFragment> {
    let mut candidates: Vec<&FloraFragment> = flora
        .fragments
        .iter()
        .filter(|f| f.slot == slot)
        .filter(|f| pool.is_none_or(|p| f.pool == p || p == "any"))
        .filter(|f| {
            if let Some(sid) = style_id {
                f.style_affinity.is_empty() || f.style_affinity.iter().any(|a| a == sid)
            } else {
                true
            }
        })
        .filter(|f| !already.iter().any(|t| t.eq_ignore_ascii_case(&f.text)))
        .collect();
    candidates.sort_by(|a, b| b.weight.cmp(&a.weight));
    candidates.into_iter().take(max).cloned().collect()
}

pub fn assemble_live(
    skeleton: &Skeleton,
    style_id: Option<&str>,
    styles: &StylesFile,
    flora: &FloraFile,
    extra: Option<&str>,
    pool: Option<&str>,
) -> (String, Vec<String>) {
    let mut parts: Vec<String> = vec![];
    let mut used_flora: Vec<String> = vec![];

    if !skeleton.subject.trim().is_empty() {
        parts.push(skeleton.subject.trim().to_string());
    }
    if !skeleton.action.trim().is_empty() {
        parts.push(skeleton.action.trim().to_string());
    }
    if !skeleton.setting.trim().is_empty() {
        parts.push(skeleton.setting.trim().to_string());
    }

    if let Some(sid) = style_id {
        if let Some(pack) = styles.styles.iter().find(|s| s.id == sid) {
            if let Some(p0) = pack.phrases.first() {
                parts.push(p0.clone());
            }
        }
    }

    let already: Vec<String> = parts.clone();
    let lighting = pick_flora(flora, style_id, "lighting", pool, 1, &already);
    let medium = pick_flora(flora, style_id, "medium", pool, 1, &already);
    for f in lighting.into_iter().chain(medium) {
        used_flora.push(f.id.clone());
        parts.push(f.text.clone());
    }

    if let Some(ex) = extra {
        if !ex.trim().is_empty() {
            parts.push(ex.trim().to_string());
        }
    }

    let mut seen = HashSet::new();
    let mut uniq = vec![];
    for p in parts {
        let k = p.to_lowercase();
        if seen.insert(k) {
            uniq.push(p);
        }
    }
    (uniq.join(", "), used_flora)
}

pub fn roulette_mashup(lib: &Library, styles: &StylesFile) -> Option<(String, String, String)> {
    if lib.prompts.is_empty() || styles.styles.is_empty() {
        return None;
    }
    let p = &lib.prompts[rand_index(lib.prompts.len())];
    let s = &styles.styles[rand_index(styles.styles.len())];
    let core: Vec<&str> = p
        .prompt
        .split(',')
        .map(str::trim)
        .take(3)
        .filter(|c| !c.is_empty())
        .collect();
    let phrase = s.phrases.first().map(|x| x.as_str()).unwrap_or(s.name.as_str());
    let mut clauses = core;
    clauses.push(phrase);
    if let Some(p2) = s.phrases.get(1) {
        clauses.push(p2.as_str());
    }
    Some((p.title.clone(), s.name.clone(), clauses.join(", ")))
}

fn rand_index(len: usize) -> usize {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as usize)
        .unwrap_or(1);
    t % len.max(1)
}

pub fn new_prompt_entry(title: &str, prompt: &str) -> PromptEntry {
    let now = now_iso();
    let mut entry = PromptEntry {
        id: new_id(),
        title: if title.trim().is_empty() {
            prompt.chars().take(48).collect()
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

/// Absolute path to the card image, if any (explicit field or media/{id}.*).
pub fn resolve_prompt_image(p: &PromptEntry) -> Option<PathBuf> {
    let media = pack_dir(&p.pack_id).join("media");
    if let Some(name) = p.image.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let path = if Path::new(name).is_absolute() {
            PathBuf::from(name)
        } else {
            media.join(name)
        };
        if path.is_file() {
            return Some(path);
        }
    }
    for name in &p.images {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let path = media.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    for ext in ["webp", "png", "jpg", "jpeg", "gif", "svg"] {
        let path = media.join(format!("{}.{}", p.id, ext));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// `file://` URL for webview `<img src>`.
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

/// Deck display title from pack meta, else slug title-case.
pub fn deck_title(lib: &Library, pack_id: &str) -> String {
    lib.packs
        .get(pack_id)
        .map(|m| m.title.clone())
        .unwrap_or_else(|| title_case_slug(pack_id))
}

pub fn sort_prompt_indices(lib: &Library) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..lib.prompts.len()).collect();
    idx.sort_by(|&a, &b| {
        let pa = &lib.prompts[a];
        let pb = &lib.prompts[b];
        let score = |p: &PromptEntry| -> i32 {
            let mut s = 0;
            // Prefer cards that have a browsable image.
            if resolve_prompt_image(p).is_some() {
                s += 40;
            }
            if p.needs_rework {
                s += 20;
            }
            if p.last_outcome.is_none() && p.copy_count_without_scar > 0 {
                s += 50;
            }
            if p.storage == "hot" {
                s += 10;
            }
            if p.storage == "compost" {
                s -= 20;
            }
            s
        };
        score(pb)
            .cmp(&score(pa))
            .then_with(|| pb.updated_at.cmp(&pa.updated_at))
    });
    idx
}

/// List pack ids present on disk or in memory.
pub fn list_pack_ids(lib: &Library) -> Vec<String> {
    let mut set: HashSet<String> = lib.packs.keys().cloned().collect();
    for p in &lib.prompts {
        set.insert(p.pack_id.clone());
    }
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_packs_and_catalog() {
        let lib = load_library().expect("load");
        assert!(lib.prompts.len() >= 3, "expected starter prompts, got {}", lib.prompts.len());
        assert!(lib.prompts.iter().any(|p| p.pack_id == "murdoch-core"));
        assert!(lib.next_experiment.is_some());
        rebuild_catalog_for_test(&lib);
    }

    fn rebuild_catalog_for_test(lib: &Library) {
        crate::catalog::rebuild(lib).expect("catalog");
        let ids = crate::catalog::search(&crate::catalog::SearchQuery {
            text: "pitbull".into(),
            limit: 10,
            ..Default::default()
        })
        .expect("search");
        assert_eq!(ids, vec!["starter-mucha-pitbull".to_string()]);
        let ids2 = crate::catalog::search(&crate::catalog::SearchQuery {
            text: "mucha".into(),
            limit: 10,
            ..Default::default()
        })
        .expect("search mucha");
        assert!(ids2.contains(&"starter-mucha-pitbull".to_string()));
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
        let lib = load_library().expect("load");
        let p = lib
            .prompts
            .iter()
            .find(|p| p.id == "starter-pc98-wordsmiths")
            .expect("starter");
        let path = resolve_prompt_image(p).expect("media file");
        assert!(path.extension().and_then(|e| e.to_str()) == Some("svg"));
        let url = prompt_image_url(p).expect("url");
        assert!(url.starts_with("file://"));
    }
}
