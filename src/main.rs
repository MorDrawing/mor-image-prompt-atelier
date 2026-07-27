mod catalog;
mod library;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use catalog::{SearchQuery, facet_counts};
use library::{
    assemble_live, bump_flora_for_prompt, find_cousins, infer_subject_class, list_pack_ids,
    load_flora, load_library, load_styles, new_prompt_entry, now_iso, rework_count, roulette_mashup,
    save_flora, save_library, sort_prompt_indices, ExperimentHistoryItem, FloraFile, Library,
    NextExperiment, PromptEntry, StylesFile,
};

static APP_CSS: &str = include_str!("../assets/style.css");

/// Persist library (packs + desk + catalog) and update status.
fn commit(lib: &mut Signal<Library>, status: &mut Signal<String>, l: Library, ok: impl Into<String>) {
    match save_library(&l) {
        Ok(()) => {
            let n = l.prompts.len();
            lib.set(l);
            status.set(format!("{} · {} prompts indexed", ok.into(), n));
        }
        Err(e) => status.set(e),
    }
}

fn main() {
    let cfg = Config::new()
        .with_menu(None::<dioxus::desktop::muda::Menu>)
        .with_window(
            WindowBuilder::new()
                .with_title("Mor Image Prompt Atelier")
                .with_inner_size(LogicalSize::new(1380.0, 860.0)),
        );

    LaunchBuilder::new().with_cfg(cfg).launch(App);
}

fn copy_to_clipboard(text: &str) {
    let payload = serde_json::Value::from(text).to_string();
    let js = format!(
        r#"(function (t) {{
    function fallback(t) {{
        var ta = document.createElement('textarea');
        ta.value = t;
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.select();
        try {{ document.execCommand('copy'); }} catch (e) {{}}
        document.body.removeChild(ta);
    }}
    if (navigator.clipboard && navigator.clipboard.writeText) {{
        navigator.clipboard.writeText(t).catch(function () {{ fallback(t); }});
    }} else {{
        fallback(t);
    }}
}})({payload});"#
    );
    let _ = dioxus::document::eval(&js);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterMode {
    All,
    Rework,
    PendingScar,
    Won,
    Cold,
}

#[derive(Clone)]
struct AppData {
    lib: Library,
    styles: StylesFile,
    flora: FloraFile,
    load_error: Option<String>,
}

fn load_app_data() -> AppData {
    let (mut lib, mut load_error) = match load_library() {
        Ok(l) => (l, None),
        Err(e) => (Library::default(), Some(e)),
    };
    // Ensure packs + catalog exist after first open of legacy data.
    if let Err(e) = save_library(&lib) {
        load_error = Some(load_error.unwrap_or_default() + " | " + &e);
    } else if let Ok(reloaded) = load_library() {
        lib = reloaded;
    }
    let _ = catalog::ensure_index(&lib);
    AppData {
        lib,
        styles: load_styles().unwrap_or_default(),
        flora: load_flora().unwrap_or_default(),
        load_error,
    }
}

#[component]
fn App() -> Element {
    let initial = use_hook(load_app_data);
    let mut lib = use_signal(|| initial.lib.clone());
    let styles = use_signal(|| initial.styles.clone());
    let mut flora = use_signal(|| initial.flora.clone());
    let mut selected_id = use_signal(|| {
        initial
            .lib
            .next_experiment
            .as_ref()
            .filter(|e| e.status == "open")
            .map(|e| e.prompt_id.clone())
            .or_else(|| initial.lib.prompts.first().map(|p| p.id.clone()))
            .unwrap_or_default()
    });
    let mut filter = use_signal(|| FilterMode::All);
    let mut query = use_signal(String::new);
    let mut pack_filter = use_signal(|| String::from("all"));
    let mut class_filter = use_signal(|| String::from("all"));
    let mut status = use_signal(|| {
        initial
            .load_error
            .clone()
            .unwrap_or_else(|| {
                let packs = list_pack_ids(&initial.lib).len();
                format!(
                    "Packs: {} · prompts: {} · catalog ready",
                    packs,
                    initial.lib.prompts.len()
                )
            })
    });
    let mut cold_nag = use_signal(|| false);
    let mut disposition_note = use_signal(String::new);
    let mut cousin_warn = use_signal(String::new);
    let mut roulette_preview = use_signal(String::new);
    let mut draft_title = use_signal(String::new);
    let mut draft_prompt = use_signal(String::new);
    let mut draft_notes = use_signal(String::new);
    let mut draft_tags = use_signal(String::new);
    let mut draft_tier = use_signal(String::new);
    let mut draft_pack = use_signal(String::new);
    let mut experiment_note = use_signal(String::new);

    let mut sync_drafts = move |entry: &PromptEntry| {
        draft_title.set(entry.title.clone());
        draft_prompt.set(entry.prompt.clone());
        draft_notes.set(entry.notes.clone());
        draft_tags.set(entry.tags.join(", "));
        draft_tier.set(entry.tier.clone());
        draft_pack.set(entry.pack_id.clone());
        disposition_note.set(entry.last_note.clone());
        cold_nag.set(false);
        cousin_warn.set(String::new());
    };

    {
        let l = lib();
        if let Some(p) = l.prompts.iter().find(|p| p.id == selected_id()) {
            if draft_prompt().is_empty() && !p.prompt.is_empty() {
                draft_title.set(p.title.clone());
                draft_prompt.set(p.prompt.clone());
                draft_notes.set(p.notes.clone());
                draft_tags.set(p.tags.join(", "));
                draft_tier.set(p.tier.clone());
                draft_pack.set(p.pack_id.clone());
                disposition_note.set(p.last_note.clone());
            }
        }
        if let Some(ne) = &l.next_experiment {
            if experiment_note().is_empty() {
                experiment_note.set(ne.note.clone());
            }
        }
    }

    let mut select_prompt = move |id: String| {
        selected_id.set(id.clone());
        if let Some(p) = lib().prompts.iter().find(|p| p.id == id) {
            sync_drafts(p);
        }
    };

    let mut save_current = move |_| {
        let id = selected_id();
        let mut l = lib();
        let Some(idx) = l.prompts.iter().position(|p| p.id == id) else {
            status.set("No prompt selected".into());
            return;
        };
        let prompt_text = draft_prompt();
        let cousins = find_cousins(&l, &prompt_text, Some(&id));
        if !cousins.is_empty() {
            let msg = cousins
                .iter()
                .take(3)
                .map(|(cid, title, sim)| {
                    let pct = (sim * 100.0).round() as i32;
                    let short = cid.chars().take(8).collect::<String>();
                    format!("{title} ({short}… {pct}%)")
                })
                .collect::<Vec<_>>()
                .join("; ");
            cousin_warn.set(format!("Near-duplicates: {msg}. Merge or diverge intentionally."));
        } else {
            cousin_warn.set(String::new());
        }

        let tags: Vec<String> = draft_tags()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        l.prompts[idx].title = draft_title();
        l.prompts[idx].prompt = prompt_text.clone();
        l.prompts[idx].notes = draft_notes();
        l.prompts[idx].tags = tags;
        l.prompts[idx].tier = draft_tier();
        let pack = draft_pack().trim().to_string();
        l.prompts[idx].pack_id = if pack.is_empty() {
            "inbox".into()
        } else {
            library::sanitize_pack_id(&pack)
        };
        l.prompts[idx].subject_class = Some(infer_subject_class(&l.prompts[idx]));
        l.prompts[idx].updated_at = now_iso();

        let frag_ids = l.prompts[idx].fragment_ids.clone();
        let tier = l.prompts[idx].tier.clone();
        let mut fl = flora();
        let delta = match tier.as_str() {
            "SS" => 2,
            "S" => 1,
            "A" => 0,
            "B" => -1,
            "C" => -2,
            _ => 0,
        };
        if delta != 0 && !frag_ids.is_empty() {
            bump_flora_for_prompt(&mut fl, &frag_ids, delta);
            let _ = save_flora(&fl);
            flora.set(fl);
        }

        let pack_label = l.prompts[idx].pack_id.clone();
        commit(
            &mut lib,
            &mut status,
            l,
            format!("Saved → packs/{pack_label}"),
        );
    };

    let mut on_copy = move |_| {
        let text = draft_prompt();
        if text.trim().is_empty() {
            status.set("Nothing to copy".into());
            return;
        }
        copy_to_clipboard(&text);
        let id = selected_id();
        let mut l = lib();
        if let Some(p) = l.prompts.iter_mut().find(|p| p.id == id) {
            p.last_run_at = Some(now_iso());
            if p.last_outcome.is_none() {
                p.copy_count_without_scar = p.copy_count_without_scar.saturating_add(1);
                if p.copy_count_without_scar >= 2 {
                    cold_nag.set(true);
                }
            }
            p.updated_at = now_iso();
        }
        let note = format!("Log outcome for: {}", draft_title());
        l.next_experiment = Some(NextExperiment {
            prompt_id: id.clone(),
            action: "rework".into(),
            note: note.clone(),
            status: "open".into(),
            updated_at: now_iso(),
        });
        experiment_note.set(note);
        commit(
            &mut lib,
            &mut status,
            l,
            "Copied — mark won/failed when the image returns",
        );
    };

    let mut dispose = move |outcome: &'static str| {
        let id = selected_id();
        let note = disposition_note();
        let mut l = lib();
        let Some(idx) = l.prompts.iter().position(|p| p.id == id) else {
            return;
        };
        let now = now_iso();
        l.prompts[idx].last_outcome = Some(outcome.into());
        l.prompts[idx].last_note = note.clone();
        l.prompts[idx].last_disposition_at = Some(now.clone());
        l.prompts[idx].last_run_at = Some(now.clone());
        l.prompts[idx].copy_count_without_scar = 0;
        l.prompts[idx].updated_at = now.clone();

        match outcome {
            "failed" | "ambiguous" => {
                l.prompts[idx].needs_rework = true;
                l.next_experiment = Some(NextExperiment {
                    prompt_id: id.clone(),
                    action: "rework".into(),
                    note: if note.is_empty() {
                        format!("Rework after {outcome}")
                    } else {
                        note.clone()
                    },
                    status: "open".into(),
                    updated_at: now.clone(),
                });
            }
            "won" => {
                l.prompts[idx].needs_rework = false;
                let frag_ids = l.prompts[idx].fragment_ids.clone();
                let mut fl = flora();
                bump_flora_for_prompt(&mut fl, &frag_ids, 2);
                let _ = save_flora(&fl);
                flora.set(fl);
            }
            _ => {}
        }
        cold_nag.set(false);
        commit(&mut lib, &mut status, l, format!("Scar recorded: {outcome}"));
    };

    let mut new_prompt = move |_| {
        let entry = new_prompt_entry("Untitled prompt", "");
        let id = entry.id.clone();
        let mut l = lib();
        l.prompts.insert(0, entry);
        l.next_experiment = Some(NextExperiment {
            prompt_id: id.clone(),
            action: "custom".into(),
            note: "Draft a subject-first prompt, then copy and scar.".into(),
            status: "open".into(),
            updated_at: now_iso(),
        });
        match save_library(&l) {
            Ok(()) => {
                lib.set(l);
                selected_id.set(id);
                draft_title.set("Untitled prompt".into());
                draft_prompt.set(String::new());
                draft_notes.set(String::new());
                draft_tags.set(String::new());
                draft_tier.set("B".into());
                draft_pack.set("inbox".into());
                disposition_note.set(String::new());
                status.set("New prompt → packs/inbox".into());
            }
            Err(e) => status.set(e),
        }
    };

    let mut complete_experiment = move |done: bool| {
        let mut l = lib();
        if let Some(ne) = l.next_experiment.take() {
            l.experiment_history.insert(
                0,
                ExperimentHistoryItem {
                    prompt_id: ne.prompt_id,
                    action: ne.action,
                    note: experiment_note(),
                    status: if done {
                        "done".into()
                    } else {
                        "dismissed".into()
                    },
                    closed_at: now_iso(),
                },
            );
            l.experiment_history.truncate(40);
        }
        match save_library(&l) {
            Ok(()) => {
                lib.set(l);
                experiment_note.set(String::new());
                status.set(if done {
                    "Experiment complete".into()
                } else {
                    "Experiment dismissed".into()
                });
            }
            Err(e) => status.set(e),
        }
    };

    let mut set_experiment_from_current = move |_| {
        let id = selected_id();
        let mut l = lib();
        l.next_experiment = Some(NextExperiment {
            prompt_id: id,
            action: "custom".into(),
            note: experiment_note(),
            status: "open".into(),
            updated_at: now_iso(),
        });
        commit(&mut lib, &mut status, l, "What's next updated");
    };

    let mut run_roulette = move |_| {
        let l = lib();
        let s = styles();
        match roulette_mashup(&l, &s) {
            Some((title, style_name, mash)) => {
                roulette_preview.set(format!("{title} × {style_name}\n{mash}"));
                copy_to_clipboard(&mash);
                status.set(format!("Roulette: {title} × {style_name} (copied)"));
            }
            None => status.set("Roulette needs prompts + styles".into()),
        }
    };

    let mut ship_jit = move |_| {
        let id = selected_id();
        let l = lib();
        let Some(p) = l.prompts.iter().find(|p| p.id == id) else {
            status.set("Select a prompt with a skeleton".into());
            return;
        };
        let skeleton = match &p.skeleton {
            Some(sk) if !sk.subject.trim().is_empty() => sk.clone(),
            _ => {
                status.set("No skeleton — add subject/action/setting via MCP or JSON".into());
                return;
            }
        };
        let style_id = styles()
            .styles
            .iter()
            .find(|s| p.tags.iter().any(|t| t == &s.id))
            .map(|s| s.id.clone())
            .or_else(|| {
                p.tags
                    .iter()
                    .find(|t| {
                        styles()
                            .styles
                            .iter()
                            .any(|s| s.id == **t || s.tags.iter().any(|x| x == *t))
                    })
                    .cloned()
            });

        let pool = if p.tags.iter().any(|t| t == "mucha" || t == "art-nouveau") {
            Some("experimental")
        } else {
            Some("murdoch-core")
        };

        let (assembled, used) = assemble_live(
            &skeleton,
            style_id.as_deref(),
            &styles(),
            &flora(),
            None,
            pool,
        );
        draft_prompt.set(assembled.clone());
        copy_to_clipboard(&assembled);
        let flora_label = if used.is_empty() {
            "0".to_string()
        } else {
            used.join(", ")
        };
        status.set(format!("JIT ship ({flora_label} flora): copied"));
    };

    let mut promote_or_cull = move |to: &'static str| {
        let id = selected_id();
        let mut l = lib();
        if let Some(p) = l.prompts.iter_mut().find(|p| p.id == id) {
            p.storage = to.into();
            p.updated_at = now_iso();
            if to == "compost" {
                p.needs_rework = false;
            }
        }
        commit(&mut lib, &mut status, l, format!("Storage → {to}"));
    };

    let sorted = sort_prompt_indices(&lib());
    let q = query();
    let filt = filter();
    let pf = pack_filter();
    let cf = class_filter();

    // FTS catalog when typing; otherwise sort order + facet filters.
    let catalog_ids: Option<Vec<String>> = if q.trim().is_empty() {
        None
    } else {
        let sq = SearchQuery {
            text: q.clone(),
            pack_id: if pf == "all" {
                None
            } else {
                Some(pf.clone())
            },
            subject_class: if cf == "all" {
                None
            } else {
                Some(cf.clone())
            },
            limit: 500,
            ..Default::default()
        };
        match catalog::search(&sq) {
            Ok(ids) if !ids.is_empty() => Some(ids),
            Ok(_) | Err(_) => Some(catalog::filter_in_memory(&lib(), &sq)),
        }
    };

    let visible: Vec<usize> = sorted
        .into_iter()
        .filter(|&i| {
            let p = &lib().prompts[i];
            let pass_filter = match filt {
                FilterMode::All => p.storage != "compost",
                FilterMode::Rework => p.needs_rework,
                FilterMode::PendingScar => p.last_outcome.is_none() && p.copy_count_without_scar > 0,
                FilterMode::Won => p.last_outcome.as_deref() == Some("won"),
                FilterMode::Cold => p.storage == "cold" || p.storage == "compost",
            };
            if !pass_filter {
                return false;
            }
            if pf != "all" && p.pack_id != pf {
                return false;
            }
            if cf != "all" && infer_subject_class(p) != cf {
                return false;
            }
            if let Some(ref ids) = catalog_ids {
                return ids.iter().any(|id| id == &p.id);
            }
            true
        })
        .collect();

    let current = lib()
        .prompts
        .iter()
        .find(|p| p.id == selected_id())
        .cloned();
    let next_exp = lib().next_experiment.clone();
    let rework_n = rework_count(&lib());
    let pending_n = lib()
        .prompts
        .iter()
        .filter(|p| p.last_outcome.is_none() && p.copy_count_without_scar > 0)
        .count();
    let pack_ids = list_pack_ids(&lib());
    let facets = facet_counts().unwrap_or_default();

    rsx! {
        style { {APP_CSS} }

        div { class: "editor-shell",
            if let Some(ne) = next_exp.clone().filter(|e| e.status == "open") {
                div { class: "resume-strip",
                    div { class: "resume-main",
                        span { class: "resume-label", "What's next" }
                        span { class: "resume-note", "{ne.note}" }
                    }
                    div { class: "resume-actions",
                        button {
                            class: "strip-btn",
                            onclick: move |_| select_prompt(ne.prompt_id.clone()),
                            "Open"
                        }
                        button {
                            class: "strip-btn primary",
                            onclick: move |_| complete_experiment(true),
                            "Done"
                        }
                        button {
                            class: "strip-btn",
                            onclick: move |_| complete_experiment(false),
                            "Dismiss"
                        }
                    }
                }
            }

            if cold_nag() {
                div { class: "nag-banner",
                    "Cold copy — log last run (Won / Failed / Ambiguous) so the desk can learn."
                    button {
                        class: "strip-btn",
                        onclick: move |_| cold_nag.set(false),
                        "Later"
                    }
                }
            }

            main { class: "app-shell",
                aside { class: "sidebar",
                    h1 { "Prompt Cabinet" }
                    p { class: "subtitle",
                        "Packs · FTS · scar loop. Not a graveyard of strings."
                    }

                    div { class: "sidebar-stats",
                        span { "Rework {rework_n}" }
                        span { "Pending scars {pending_n}" }
                        span { "Indexed {facets.total}" }
                    }

                    input {
                        class: "search-input",
                        placeholder: "FTS: title, tags, prompt, class…",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }

                    div { class: "filter-row",
                        button {
                            class: if filt == FilterMode::All { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(FilterMode::All),
                            "All"
                        }
                        button {
                            class: if filt == FilterMode::Rework { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(FilterMode::Rework),
                            "Rework"
                        }
                        button {
                            class: if filt == FilterMode::PendingScar { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(FilterMode::PendingScar),
                            "Scars?"
                        }
                        button {
                            class: if filt == FilterMode::Won { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(FilterMode::Won),
                            "Won"
                        }
                        button {
                            class: if filt == FilterMode::Cold { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(FilterMode::Cold),
                            "Cold"
                        }
                    }

                    div { class: "filter-row pack-row",
                        button {
                            class: if pf == "all" { "chip active" } else { "chip" },
                            onclick: move |_| pack_filter.set("all".into()),
                            "Packs"
                        }
                        for pid in pack_ids.iter().cloned() {
                            {
                                let pid2 = pid.clone();
                                let active = pf == pid;
                                rsx! {
                                    button {
                                        class: if active { "chip active" } else { "chip" },
                                        onclick: move |_| pack_filter.set(pid2.clone()),
                                        "{pid}"
                                    }
                                }
                            }
                        }
                    }

                    div { class: "filter-row",
                        button {
                            class: if cf == "all" { "chip active" } else { "chip" },
                            onclick: move |_| class_filter.set("all".into()),
                            "Class"
                        }
                        for (cls, _) in [
                            ("character", ()),
                            ("animal", ()),
                            ("scene", ()),
                            ("poster", ()),
                            ("other", ()),
                        ] {
                            {
                                let c = cls.to_string();
                                let active = cf == cls;
                                rsx! {
                                    button {
                                        class: if active { "chip active" } else { "chip" },
                                        onclick: move |_| class_filter.set(c.clone()),
                                        "{cls}"
                                    }
                                }
                            }
                        }
                    }

                    nav { class: "prompt-list",
                        for i in visible {
                            {
                                let p = lib().prompts[i].clone();
                                let id = p.id.clone();
                                let active = selected_id() == id;
                                let scar = p.last_outcome.clone().unwrap_or_else(|| "—".into());
                                let klass = infer_subject_class(&p);
                                rsx! {
                                    button {
                                        class: if active { "prompt-button active" } else { "prompt-button" },
                                        onclick: move |_| select_prompt(id.clone()),
                                        span { class: "prompt-title", "{p.title}" }
                                        span { class: "prompt-meta",
                                            "{p.pack_id} · {klass} · tier {p.tier} · {scar}"
                                            if p.needs_rework {
                                                " · rework"
                                            }
                                            if p.storage != "hot" {
                                                " · {p.storage}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button { class: "copy-button sidebar-new", onclick: move |_| new_prompt(()), "+ New prompt" }
                }

                section { class: "editor-panel",
                    if let Some(cur) = current.clone() {
                        div { class: "panel-header",
                            div {
                                h2 { "{cur.title}" }
                                p { class: "muted-line",
                                    "id {cur.id} · pack {cur.pack_id} · {infer_subject_class(&cur)} · storage {cur.storage}"
                                }
                            }
                            div { class: "header-actions",
                                button { class: "copy-button", onclick: move |_| save_current(()), "Save" }
                                button { class: "copy-button primary-btn", onclick: move |_| on_copy(()), "Copy Prompt" }
                                button { class: "copy-button", onclick: move |_| ship_jit(()), "JIT Ship" }
                            }
                        }

                        div { class: "tag-row",
                            for tag in cur.tags.iter() {
                                span { class: "tag", "#{tag}" }
                            }
                            if let Some(out) = &cur.last_outcome {
                                span { class: "tag scar-tag scar-{out}", "scar:{out}" }
                            }
                        }

                        if !cousin_warn().is_empty() {
                            div { class: "cousin-warn", "{cousin_warn}" }
                        }

                        label { class: "field-label", "Title" }
                        input {
                            class: "text-input",
                            value: "{draft_title}",
                            oninput: move |e| draft_title.set(e.value()),
                        }

                        label { class: "field-label", "Tier (SS–C)" }
                        input {
                            class: "text-input narrow",
                            value: "{draft_tier}",
                            oninput: move |e| draft_tier.set(e.value()),
                        }

                        label { class: "field-label", "Pack id" }
                        input {
                            class: "text-input",
                            placeholder: "murdoch-core, characters, poster-icons, inbox…",
                            value: "{draft_pack}",
                            oninput: move |e| draft_pack.set(e.value()),
                        }

                        label { class: "field-label", "Tags (comma-separated)" }
                        input {
                            class: "text-input",
                            value: "{draft_tags}",
                            oninput: move |e| draft_tags.set(e.value()),
                        }

                        label { class: "field-label", "Prompt" }
                        textarea {
                            class: "prompt-textarea",
                            value: "{draft_prompt}",
                            oninput: move |e| draft_prompt.set(e.value()),
                        }

                        label { class: "field-label", "Notes" }
                        textarea {
                            class: "notes-textarea",
                            value: "{draft_notes}",
                            oninput: move |e| draft_notes.set(e.value()),
                        }

                        div { class: "returns-desk",
                            h3 { "Returns desk" }
                            p { class: "muted-line",
                                "After you generate externally, scar the run so rework has a target."
                            }
                            label { class: "field-label", "Outcome note" }
                            input {
                                class: "text-input",
                                placeholder: "too muddy / lost subject / perfect mood…",
                                value: "{disposition_note}",
                                oninput: move |e| disposition_note.set(e.value()),
                            }
                            div { class: "returns-row",
                                button {
                                    class: "outcome-btn won",
                                    onclick: move |_| dispose("won"),
                                    "Won"
                                }
                                button {
                                    class: "outcome-btn failed",
                                    onclick: move |_| dispose("failed"),
                                    "Failed"
                                }
                                button {
                                    class: "outcome-btn ambiguous",
                                    onclick: move |_| dispose("ambiguous"),
                                    "Ambiguous"
                                }
                            }
                            if let Some(out) = &cur.last_outcome {
                                p { class: "muted-line",
                                    "Last scar: {out}"
                                    if !cur.last_note.is_empty() {
                                        " — {cur.last_note}"
                                    }
                                }
                            }
                        }

                        div { class: "experiment-box",
                            h3 { "Pin as What's next" }
                            input {
                                class: "text-input",
                                placeholder: "Concrete next action…",
                                value: "{experiment_note}",
                                oninput: move |e| experiment_note.set(e.value()),
                            }
                            button {
                                class: "copy-button",
                                onclick: move |_| set_experiment_from_current(()),
                                "Set mission"
                            }
                        }

                        div { class: "storage-row",
                            button { class: "chip", onclick: move |_| promote_or_cull("hot"), "Promote hot" }
                            button { class: "chip", onclick: move |_| promote_or_cull("cold"), "Cold storage" }
                            button { class: "chip", onclick: move |_| promote_or_cull("compost"), "Compost" }
                        }
                    } else {
                        div { class: "empty-state",
                            h2 { "Empty cabinet" }
                            p { "Create a prompt — stored under data/packs/<pack>/prompts/" }
                            button { class: "copy-button", onclick: move |_| new_prompt(()), "New prompt" }
                        }
                    }

                    p { class: "status-line", "{status}" }
                }

                aside { class: "image-panel ops-panel",
                    h2 { "Desk ops" }

                    div { class: "ops-card",
                        h3 { "Roulette" }
                        p { class: "muted-line", "Random library core × style pack — serendipity without forms." }
                        button { class: "copy-button", onclick: move |_| run_roulette(()), "Spin & copy" }
                        if !roulette_preview().is_empty() {
                            pre { class: "roulette-pre", "{roulette_preview}" }
                        }
                    }

                    div { class: "ops-card",
                        h3 { "Flora ({flora().fragments.len()})" }
                        p { class: "muted-line",
                            "Micro-fragments for JIT ship (max 2 slots). Weights shift with tiers & wins."
                        }
                        ul { class: "flora-list",
                            for f in flora().fragments.iter().take(8) {
                                li {
                                    span { class: "flora-id", "{f.id}" }
                                    span { class: "flora-w", "w{f.weight}" }
                                    span { class: "flora-text", "{f.text}" }
                                }
                            }
                        }
                    }

                    div { class: "ops-card",
                        h3 { "Storage layout" }
                        p { class: "muted-line",
                            "Packs (content) + desk.json (missions) + catalog.sqlite (FTS). Flat library.json is an export."
                        }
                        small { "{library::packs_dir().display()}" }
                        if !facets.packs.is_empty() {
                            ul { class: "flora-list",
                                for (pack, n) in facets.packs.iter().take(8) {
                                    li {
                                        span { class: "flora-id", "{pack}" }
                                        span { class: "flora-w", "{n}" }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "ops-card",
                        h3 { "MCP craft" }
                        p { class: "muted-line",
                            "Agents use mor-image-prompts: critique, improve, build, vary, record_outcome, list_packs, list_flora."
                        }
                        small { "{library::desk_path().display()}" }
                    }

                    if !lib().experiment_history.is_empty() {
                        div { class: "ops-card",
                            h3 { "Mission history" }
                            ul { class: "history-list",
                                for h in lib().experiment_history.iter().take(6) {
                                    li {
                                        "{h.status}: {h.action} — {h.note}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
