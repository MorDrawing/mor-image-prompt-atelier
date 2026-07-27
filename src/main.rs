mod catalog;
mod library;

use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{icon_from_memory, Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use catalog::{SearchQuery, facet_counts};
use library::{
    assemble_live, bump_flora_for_prompt, deck_title, find_cousins, infer_subject_class,
    list_pack_ids, load_flora, load_library, load_styles, new_prompt_entry, now_iso,
    prompt_image_url, rework_count, roulette_mashup, save_flora, save_library, sort_prompt_indices,
    FloraFile, Library, PromptEntry, StylesFile,
};

static APP_CSS: &str = include_str!("../assets/style.css");
/// Window / taskbar icon (embedded so it is not the Dioxus default).
static APP_ICON_PNG: &[u8] =
    include_bytes!("../assets/icons/hicolor/256x256/apps/mor-image-prompt-atelier.png");

/// Persist library (packs + desk + catalog) and update status.
fn commit(lib: &mut Signal<Library>, status: &mut Signal<String>, l: Library, ok: impl Into<String>) {
    match save_library(&l) {
        Ok(()) => {
            let n = l.prompts.len();
            lib.set(l);
            status.set(format!("{} · {} cards", ok.into(), n));
        }
        Err(e) => status.set(e),
    }
}

fn load_window_icon() -> Option<Icon> {
    icon_from_memory::<Icon>(APP_ICON_PNG).ok()
}

fn main() {
    let mut cfg = Config::new()
        .with_menu(None::<dioxus::desktop::muda::Menu>)
        .with_window(
            WindowBuilder::new()
                .with_title("Mor Image Prompt Atelier")
                .with_inner_size(LogicalSize::new(1440.0, 900.0)),
        );
    // Without this, dioxus-desktop injects its own default window icon.
    if let Some(icon) = load_window_icon() {
        cfg = cfg.with_icon(icon);
    }

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

fn prompt_preview(text: &str, max: usize) -> String {
    let t = text.trim().replace('\n', " ");
    if t.chars().count() <= max {
        return t;
    }
    let mut s: String = t.chars().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
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
            .prompts
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default()
    });
    let mut query = use_signal(String::new);
    let mut deck_filter = use_signal(|| String::from("all"));
    let mut class_filter = use_signal(|| String::from("all"));
    let mut status = use_signal(|| {
        initial.load_error.clone().unwrap_or_else(|| {
            let decks = list_pack_ids(&initial.lib).len();
            format!(
                "{} decks · {} cards · search ready",
                decks,
                initial.lib.prompts.len()
            )
        })
    });
    let mut cousin_warn = use_signal(String::new);
    let mut roulette_preview = use_signal(String::new);
    let mut draft_title = use_signal(String::new);
    let mut draft_prompt = use_signal(String::new);
    let mut draft_notes = use_signal(String::new);
    let mut draft_tags = use_signal(String::new);
    let mut draft_tier = use_signal(String::new);
    let mut draft_pack = use_signal(String::new);
    let mut draft_image = use_signal(String::new);
    let mut craft_open = use_signal(|| !initial.lib.prompts.is_empty());

    let mut sync_drafts = move |entry: &PromptEntry| {
        draft_title.set(entry.title.clone());
        draft_prompt.set(entry.prompt.clone());
        draft_notes.set(entry.notes.clone());
        draft_tags.set(entry.tags.join(", "));
        draft_tier.set(entry.tier.clone());
        draft_pack.set(entry.pack_id.clone());
        draft_image.set(entry.image.clone().unwrap_or_default());
        cousin_warn.set(String::new());
    };

    {
        let l = lib();
        if let Some(p) = l.prompts.iter().find(|p| p.id == selected_id()) {
            if draft_prompt().is_empty() && !p.prompt.is_empty() {
                sync_drafts(p);
            }
        }
    }

    let mut select_card = move |id: String| {
        selected_id.set(id.clone());
        craft_open.set(true);
        if let Some(p) = lib().prompts.iter().find(|p| p.id == id) {
            sync_drafts(p);
        }
    };

    let mut save_current = move |_| {
        let id = selected_id();
        let mut l = lib();
        let Some(idx) = l.prompts.iter().position(|p| p.id == id) else {
            status.set("No card selected".into());
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
            cousin_warn.set(format!("Near-duplicates: {msg}"));
        } else {
            cousin_warn.set(String::new());
        }

        let tags: Vec<String> = draft_tags()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        l.prompts[idx].title = draft_title();
        l.prompts[idx].prompt = prompt_text;
        l.prompts[idx].notes = draft_notes();
        l.prompts[idx].tags = tags;
        l.prompts[idx].tier = draft_tier();
        let pack = draft_pack().trim().to_string();
        l.prompts[idx].pack_id = if pack.is_empty() {
            "inbox".into()
        } else {
            library::sanitize_pack_id(&pack)
        };
        let img = draft_image().trim().to_string();
        l.prompts[idx].image = if img.is_empty() { None } else { Some(img) };
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

        let deck = l.prompts[idx].pack_id.clone();
        commit(
            &mut lib,
            &mut status,
            l,
            format!("Saved · deck {deck}"),
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
            p.updated_at = now_iso();
        }
        commit(&mut lib, &mut status, l, "Prompt copied — paste into your generator");
    };

    let mut mark_outcome = move |outcome: &'static str| {
        let id = selected_id();
        let mut l = lib();
        let Some(idx) = l.prompts.iter().position(|p| p.id == id) else {
            return;
        };
        let now = now_iso();
        l.prompts[idx].last_outcome = Some(outcome.into());
        l.prompts[idx].last_disposition_at = Some(now.clone());
        l.prompts[idx].updated_at = now;
        if outcome == "won" {
            l.prompts[idx].needs_rework = false;
            let frag_ids = l.prompts[idx].fragment_ids.clone();
            let mut fl = flora();
            bump_flora_for_prompt(&mut fl, &frag_ids, 2);
            let _ = save_flora(&fl);
            flora.set(fl);
        } else if outcome == "failed" {
            l.prompts[idx].needs_rework = true;
        }
        commit(&mut lib, &mut status, l, format!("Marked {outcome}"));
    };

    let mut new_card = move |_| {
        let entry = new_prompt_entry("Untitled card", "");
        let id = entry.id.clone();
        let mut l = lib();
        l.prompts.insert(0, entry);
        match save_library(&l) {
            Ok(()) => {
                lib.set(l);
                selected_id.set(id);
                craft_open.set(true);
                draft_title.set("Untitled card".into());
                draft_prompt.set(String::new());
                draft_notes.set(String::new());
                draft_tags.set(String::new());
                draft_tier.set("B".into());
                draft_pack.set("inbox".into());
                draft_image.set(String::new());
                status.set("New card in inbox deck".into());
            }
            Err(e) => status.set(e),
        }
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
            None => status.set("Roulette needs cards + styles".into()),
        }
    };

    let mut ship_jit = move |_| {
        let id = selected_id();
        let l = lib();
        let Some(p) = l.prompts.iter().find(|p| p.id == id) else {
            status.set("Select a card with a skeleton".into());
            return;
        };
        let skeleton = match &p.skeleton {
            Some(sk) if !sk.subject.trim().is_empty() => sk.clone(),
            _ => {
                status.set("No skeleton on this card yet".into());
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
        status.set(format!("JIT assembled ({flora_label}) · copied"));
    };

    let sorted = sort_prompt_indices(&lib());
    let q = query();
    let df = deck_filter();
    let cf = class_filter();

    let catalog_ids: Option<Vec<String>> = if q.trim().is_empty() {
        None
    } else {
        let sq = SearchQuery {
            text: q.clone(),
            pack_id: if df == "all" {
                None
            } else {
                Some(df.clone())
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
            if p.storage == "compost" {
                return false;
            }
            if df != "all" && p.pack_id != df {
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
    let deck_ids = list_pack_ids(&lib());
    let facets = facet_counts().unwrap_or_default();
    let card_n = visible.len();
    let rework_n = rework_count(&lib());
    let craft = craft_open();

    rsx! {
        style { {APP_CSS} }

        div { class: "atelier-shell",
            header { class: "atelier-top",
                div { class: "brand",
                    h1 { "Image Prompt Atelier" }
                    p { class: "subtitle",
                        "Decks of images & prompts — search, open a card, craft, copy."
                    }
                }
                div { class: "top-tools",
                    input {
                        class: "search-input top-search",
                        placeholder: "Search title, tags, prompt, class…",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                    button { class: "copy-button", onclick: move |_| new_card(()), "+ New card" }
                    button { class: "copy-button", onclick: move |_| run_roulette(()), "Roulette" }
                }
            }

            div { class: "filter-bar",
                div { class: "filter-row",
                    span { class: "filter-label", "Decks" }
                    button {
                        class: if df == "all" { "chip active" } else { "chip" },
                        onclick: move |_| deck_filter.set("all".into()),
                        "All"
                    }
                    for pid in deck_ids.iter().cloned() {
                        {
                            let pid2 = pid.clone();
                            let label = deck_title(&lib(), &pid);
                            let active = df == pid;
                            let count = facets.packs.iter().find(|(k, _)| k == &pid).map(|(_, n)| *n).unwrap_or(0);
                            rsx! {
                                button {
                                    class: if active { "chip active" } else { "chip" },
                                    onclick: move |_| deck_filter.set(pid2.clone()),
                                    "{label}"
                                    if count > 0 {
                                        span { class: "chip-count", " {count}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "filter-row",
                    span { class: "filter-label", "Class" }
                    button {
                        class: if cf == "all" { "chip active" } else { "chip" },
                        onclick: move |_| class_filter.set("all".into()),
                        "All"
                    }
                    for cls in ["character", "animal", "scene", "poster", "other"] {
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
                    span { class: "filter-meta",
                        "{card_n} cards"
                        if rework_n > 0 {
                            " · {rework_n} rework"
                        }
                    }
                }
            }

            if !roulette_preview().is_empty() {
                div { class: "roulette-banner",
                    pre { class: "roulette-pre", "{roulette_preview}" }
                    button {
                        class: "strip-btn",
                        onclick: move |_| roulette_preview.set(String::new()),
                        "Dismiss"
                    }
                }
            }

            main { class: if craft { "atelier-body craft-open" } else { "atelier-body" },
                section { class: "deck-browser",
                    if visible.is_empty() {
                        div { class: "empty-state",
                            h2 { "No cards match" }
                            p { "Try another search, switch decks, or create a card." }
                            button { class: "copy-button", onclick: move |_| new_card(()), "New card" }
                        }
                    } else {
                        div { class: "card-grid",
                            for i in visible {
                                {
                                    let p = lib().prompts[i].clone();
                                    let id = p.id.clone();
                                    let active = selected_id() == id;
                                    let img_url = prompt_image_url(&p);
                                    let klass = infer_subject_class(&p);
                                    let deck = deck_title(&lib(), &p.pack_id);
                                    let preview = prompt_preview(&p.prompt, 110);
                                    let has_img = img_url.is_some();
                                    rsx! {
                                        button {
                                            class: if active { "deck-card active" } else { "deck-card" },
                                            onclick: move |_| select_card(id.clone()),
                                            div {
                                                class: if has_img { "card-thumb has-image" } else { "card-thumb placeholder" },
                                                if let Some(src) = img_url {
                                                    img { src: "{src}", alt: "{p.title}" }
                                                } else {
                                                    span { class: "thumb-glyph", "{klass.chars().next().unwrap_or('?')}" }
                                                    span { class: "thumb-class", "{klass}" }
                                                }
                                            }
                                            div { class: "card-body",
                                                span { class: "card-title", "{p.title}" }
                                                span { class: "card-meta",
                                                    "{deck} · {klass} · {p.tier}"
                                                    if let Some(out) = &p.last_outcome {
                                                        " · {out}"
                                                    }
                                                }
                                                span { class: "card-prompt", "{preview}" }
                                                if !p.tags.is_empty() {
                                                    div { class: "card-tags",
                                                        for tag in p.tags.iter().take(4) {
                                                            span { class: "tag mini", "#{tag}" }
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
                }

                if craft {
                    aside { class: "craft-panel",
                        if let Some(cur) = current.clone() {
                            div { class: "panel-header",
                                div {
                                    h2 { "Craft" }
                                    p { class: "muted-line",
                                        "{cur.pack_id} · {infer_subject_class(&cur)} · {cur.id}"
                                    }
                                }
                                button {
                                    class: "strip-btn",
                                    onclick: move |_| craft_open.set(false),
                                    "Hide"
                                }
                            }

                            if let Some(src) = prompt_image_url(&cur) {
                                div { class: "craft-image",
                                    img { src: "{src}", alt: "{cur.title}" }
                                }
                            } else {
                                div { class: "craft-image empty",
                                    p { "Drop a reference into" }
                                    code { "packs/{cur.pack_id}/media/" }
                                    p { "as {cur.id}.png (or set Image below)" }
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

                            div { class: "craft-row",
                                div {
                                    label { class: "field-label", "Tier" }
                                    input {
                                        class: "text-input narrow",
                                        value: "{draft_tier}",
                                        oninput: move |e| draft_tier.set(e.value()),
                                    }
                                }
                                div { class: "grow",
                                    label { class: "field-label", "Deck id" }
                                    input {
                                        class: "text-input",
                                        placeholder: "murdoch-core, characters, inbox…",
                                        value: "{draft_pack}",
                                        oninput: move |e| draft_pack.set(e.value()),
                                    }
                                }
                            }

                            label { class: "field-label", "Tags (comma-separated)" }
                            input {
                                class: "text-input",
                                value: "{draft_tags}",
                                oninput: move |e| draft_tags.set(e.value()),
                            }

                            label { class: "field-label", "Image file (in deck media/)" }
                            input {
                                class: "text-input",
                                placeholder: "my-gen.png",
                                value: "{draft_image}",
                                oninput: move |e| draft_image.set(e.value()),
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

                            div { class: "header-actions craft-actions",
                                button { class: "copy-button primary-btn", onclick: move |_| on_copy(()), "Copy prompt" }
                                button { class: "copy-button", onclick: move |_| save_current(()), "Save" }
                                button { class: "copy-button", onclick: move |_| ship_jit(()), "JIT ship" }
                            }

                            div { class: "light-outcomes",
                                span { class: "filter-label", "Optional mark" }
                                button {
                                    class: "outcome-btn won",
                                    onclick: move |_| mark_outcome("won"),
                                    "Won"
                                }
                                button {
                                    class: "outcome-btn failed",
                                    onclick: move |_| mark_outcome("failed"),
                                    "Failed"
                                }
                                button {
                                    class: "outcome-btn ambiguous",
                                    onclick: move |_| mark_outcome("ambiguous"),
                                    "Ambiguous"
                                }
                                if let Some(out) = &cur.last_outcome {
                                    span { class: "muted-line inline", "last: {out}" }
                                }
                            }
                        } else {
                            div { class: "empty-state",
                                h2 { "Pick a card" }
                                p { "Open a deck card to craft and copy its prompt." }
                            }
                        }
                        p { class: "status-line", "{status}" }
                    }
                }
            }
        }
    }
}
