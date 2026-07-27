//! SQLite FTS5 catalog for searchable classification of prompt packs.
//! Source of truth remains pack JSON + desk sidecar; this is a rebuildable index.

use crate::library::{infer_subject_class, Library, PromptEntry};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub fn catalog_path() -> PathBuf {
    crate::library::data_dir().join("catalog.sqlite")
}

pub fn rebuild(lib: &Library) -> Result<(), String> {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir catalog: {e}"))?;
    }
    // Atomic-ish rebuild: write to temp then rename.
    let tmp = path.with_extension("sqlite.tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        let conn = Connection::open(&tmp).map_err(|e| format!("open catalog tmp: {e}"))?;
        init_schema(&conn)?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("catalog tx: {e}"))?;
        {
            let mut ins = tx
                .prepare(
                    "INSERT INTO prompts (
                        id, pack_id, title, tier, storage, tags, prompt, notes,
                        last_outcome, needs_rework, subject_class, updated_at
                    ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                )
                .map_err(|e| format!("prepare insert: {e}"))?;
            let mut ins_fts = tx
                .prepare(
                    "INSERT INTO prompts_fts (
                        id, title, tags, prompt, notes, subject_class, pack_id
                    ) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                )
                .map_err(|e| format!("prepare fts insert: {e}"))?;
            for p in &lib.prompts {
                let tags = p.tags.join(" ");
                let subject_class = infer_subject_class(p);
                ins.execute(params![
                    p.id,
                    p.pack_id,
                    p.title,
                    p.tier,
                    p.storage,
                    tags,
                    p.prompt,
                    p.notes,
                    p.last_outcome.as_deref().unwrap_or(""),
                    if p.needs_rework { 1 } else { 0 },
                    subject_class,
                    p.updated_at,
                ])
                .map_err(|e| format!("insert {}: {e}", p.id))?;
                ins_fts
                    .execute(params![
                        p.id,
                        p.title,
                        tags,
                        p.prompt,
                        p.notes,
                        subject_class,
                        p.pack_id,
                    ])
                    .map_err(|e| format!("fts insert {}: {e}", p.id))?;
            }
        }
        tx.commit().map_err(|e| format!("catalog commit: {e}"))?;
    }
    std::fs::rename(&tmp, &path).map_err(|e| format!("rename catalog: {e}"))?;
    Ok(())
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE prompts (
            id TEXT PRIMARY KEY,
            pack_id TEXT NOT NULL,
            title TEXT NOT NULL,
            tier TEXT NOT NULL,
            storage TEXT NOT NULL,
            tags TEXT NOT NULL,
            prompt TEXT NOT NULL,
            notes TEXT NOT NULL,
            last_outcome TEXT NOT NULL,
            needs_rework INTEGER NOT NULL,
            subject_class TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        -- Standalone FTS (external content tables fight JOIN + MATCH aliases).
        CREATE VIRTUAL TABLE prompts_fts USING fts5(
            id UNINDEXED,
            title,
            tags,
            prompt,
            notes,
            subject_class,
            pack_id UNINDEXED
        );
        CREATE INDEX idx_prompts_pack ON prompts(pack_id);
        CREATE INDEX idx_prompts_tier ON prompts(tier);
        CREATE INDEX idx_prompts_storage ON prompts(storage);
        CREATE INDEX idx_prompts_outcome ON prompts(last_outcome);
        CREATE INDEX idx_prompts_class ON prompts(subject_class);
        "#,
    )
    .map_err(|e| format!("catalog schema: {e}"))
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub text: String,
    pub pack_id: Option<String>,
    pub tier: Option<String>,
    pub storage: Option<String>,
    pub outcome: Option<String>,
    pub subject_class: Option<String>,
    pub needs_rework: Option<bool>,
    pub limit: usize,
}

/// Ranked prompt ids from the catalog. Falls back to empty if catalog missing/corrupt.
pub fn search(q: &SearchQuery) -> Result<Vec<String>, String> {
    let path = catalog_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let conn = Connection::open(&path).map_err(|e| format!("open catalog: {e}"))?;
    let limit = if q.limit == 0 { 500 } else { q.limit };

    let fts = sanitize_fts(&q.text);
    let mut candidate_ids: Option<Vec<String>> = None;

    if !fts.is_empty() {
        let mut stmt = conn
            .prepare(
                "SELECT id FROM prompts_fts WHERE prompts_fts MATCH ?1 ORDER BY bm25(prompts_fts) LIMIT ?2",
            )
            .map_err(|e| format!("prepare fts: {e}"))?;
        let rows = stmt
            .query_map(params![fts, limit as i64], |row| row.get::<_, String>(0))
            .map_err(|e| format!("fts query: {e}"))?;
        let mut ids = vec![];
        for r in rows {
            ids.push(r.map_err(|e| format!("fts row: {e}"))?);
        }
        candidate_ids = Some(ids);
    }

    let mut sql = String::from(
        "SELECT id, pack_id, tier, storage, last_outcome, subject_class, needs_rework \
         FROM prompts",
    );
    let mut where_parts: Vec<String> = vec![];
    let mut binds: Vec<String> = vec![];

    if let Some(pack) = &q.pack_id {
        where_parts.push("pack_id = ?".into());
        binds.push(pack.clone());
    }
    if let Some(tier) = &q.tier {
        where_parts.push("tier = ?".into());
        binds.push(tier.clone());
    }
    if let Some(storage) = &q.storage {
        where_parts.push("storage = ?".into());
        binds.push(storage.clone());
    }
    if let Some(outcome) = &q.outcome {
        where_parts.push("last_outcome = ?".into());
        binds.push(outcome.clone());
    }
    if let Some(sc) = &q.subject_class {
        where_parts.push("subject_class = ?".into());
        binds.push(sc.clone());
    }
    if let Some(nr) = q.needs_rework {
        where_parts.push("needs_rework = ?".into());
        binds.push(if nr { "1".into() } else { "0".into() });
    }
    if let Some(ref ids) = candidate_ids {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        where_parts.push(format!("id IN ({placeholders})"));
        binds.extend(ids.iter().cloned());
    }

    if !where_parts.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_parts.join(" AND "));
    }
    sql.push_str(" ORDER BY updated_at DESC");
    sql.push_str(&format!(" LIMIT {limit}"));

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("prepare search: {e}"))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = binds
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| row.get::<_, String>(0))
        .map_err(|e| format!("search query: {e}"))?;

    let mut out = vec![];
    for r in rows {
        out.push(r.map_err(|e| format!("search row: {e}"))?);
    }

    // Preserve FTS rank order when we have candidates.
    if let Some(ref ranked) = candidate_ids {
        let set: std::collections::HashSet<&str> = out.iter().map(|s| s.as_str()).collect();
        out = ranked
            .iter()
            .filter(|id| set.contains(id.as_str()))
            .cloned()
            .collect();
    }
    Ok(out)
}

/// Facet counts for UI filters.
pub fn facet_counts() -> Result<FacetSummary, String> {
    let path = catalog_path();
    if !path.exists() {
        return Ok(FacetSummary::default());
    }
    let conn = Connection::open(&path).map_err(|e| format!("open catalog: {e}"))?;
    Ok(FacetSummary {
        packs: count_group(&conn, "pack_id")?,
        tiers: count_group(&conn, "tier")?,
        storage: count_group(&conn, "storage")?,
        outcomes: count_group(&conn, "last_outcome")?,
        subject_classes: count_group(&conn, "subject_class")?,
        total: conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
            .unwrap_or(0),
    })
}

fn count_group(conn: &Connection, col: &str) -> Result<Vec<(String, i64)>, String> {
    // col is internal only
    let sql = format!(
        "SELECT {col}, COUNT(*) FROM prompts GROUP BY {col} ORDER BY COUNT(*) DESC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("facet {col}: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let k: String = row.get(0)?;
            let n: i64 = row.get(1)?;
            Ok((k, n))
        })
        .map_err(|e| format!("facet map: {e}"))?;
    let mut out = vec![];
    for r in rows {
        out.push(r.map_err(|e| format!("facet row: {e}"))?);
    }
    Ok(out)
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // facets available for UI / future filters
pub struct FacetSummary {
    pub packs: Vec<(String, i64)>,
    pub tiers: Vec<(String, i64)>,
    pub storage: Vec<(String, i64)>,
    pub outcomes: Vec<(String, i64)>,
    pub subject_classes: Vec<(String, i64)>,
    pub total: i64,
}

/// Ensure catalog exists for a library (no-op if already current-ish).
pub fn ensure_index(lib: &Library) -> Result<(), String> {
    let path = catalog_path();
    if !path.exists() {
        return rebuild(lib);
    }
    // Cheap staleness check: row count mismatch → rebuild.
    let conn = Connection::open(&path).map_err(|e| format!("open catalog: {e}"))?;
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))
        .unwrap_or(-1);
    if n as usize != lib.prompts.len() {
        drop(conn);
        return rebuild(lib);
    }
    Ok(())
}

/// Escape user text for FTS5 MATCH (token OR query).
fn sanitize_fts(raw: &str) -> String {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect::<String>()
        })
        .filter(|t| t.len() >= 2)
        .map(|t| format!("\"{t}\"*"))
        .collect();
    tokens.join(" ")
}

#[allow(dead_code)]
pub fn catalog_exists() -> bool {
    Path::new(&catalog_path()).exists()
}

/// In-memory filter fallback when catalog is unavailable.
pub fn filter_in_memory(lib: &Library, q: &SearchQuery) -> Vec<String> {
    let text = q.text.to_lowercase();
    let mut ids: Vec<&PromptEntry> = lib.prompts.iter().collect();
    if let Some(pack) = &q.pack_id {
        ids.retain(|p| &p.pack_id == pack);
    }
    if let Some(tier) = &q.tier {
        ids.retain(|p| p.tier.eq_ignore_ascii_case(tier));
    }
    if let Some(storage) = &q.storage {
        ids.retain(|p| &p.storage == storage);
    }
    if let Some(outcome) = &q.outcome {
        ids.retain(|p| p.last_outcome.as_deref() == Some(outcome.as_str()));
    }
    if let Some(sc) = &q.subject_class {
        ids.retain(|p| infer_subject_class(p) == *sc);
    }
    if let Some(nr) = q.needs_rework {
        ids.retain(|p| p.needs_rework == nr);
    }
    if !text.is_empty() {
        ids.retain(|p| {
            p.title.to_lowercase().contains(&text)
                || p.prompt.to_lowercase().contains(&text)
                || p.notes.to_lowercase().contains(&text)
                || p.tags.iter().any(|t| t.to_lowercase().contains(&text))
                || p.pack_id.to_lowercase().contains(&text)
        });
    }
    ids.into_iter().map(|p| p.id.clone()).collect()
}
