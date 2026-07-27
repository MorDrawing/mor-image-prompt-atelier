// ponytail: one-paragraph prompt editor + optional result image. No scar/flora/deck chrome in UI.
mod catalog;
mod library;

use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{icon_from_memory, Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use catalog::SearchQuery;
use library::{
    load_library, new_prompt_entry, now_iso, prompt_image_url, save_library, Library, PromptEntry,
};

static APP_CSS: &str = include_str!("../assets/style.css");
static APP_ICON_PNG: &[u8] =
    include_bytes!("../assets/icons/hicolor/256x256/apps/mor-image-prompt-atelier.png");

fn load_window_icon() -> Option<Icon> {
    icon_from_memory::<Icon>(APP_ICON_PNG).ok()
}

fn main() {
    let mut cfg = Config::new()
        .with_menu(None::<dioxus::desktop::muda::Menu>)
        .with_window(
            WindowBuilder::new()
                .with_title("Mor Image Prompt Atelier")
                .with_inner_size(LogicalSize::new(1200.0, 780.0)),
        );
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

fn preview(text: &str, max: usize) -> String {
    let t = text.trim().replace('\n', " ");
    if t.chars().count() <= max {
        return t;
    }
    let mut s: String = t.chars().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
}

fn load_lib() -> (Library, Option<String>) {
    match load_library() {
        Ok(mut l) => {
            if let Err(e) = save_library(&l) {
                return (l, Some(e));
            }
            if let Ok(reloaded) = load_library() {
                l = reloaded;
            }
            let _ = catalog::ensure_index(&l);
            (l, None)
        }
        Err(e) => (Library::default(), Some(e)),
    }
}

#[component]
fn App() -> Element {
    let (initial, load_error) = use_hook(|| {
        let (lib, err) = load_lib();
        (lib, err)
    });
    let mut lib = use_signal(|| initial);
    let mut selected_id = use_signal(|| {
        lib().prompts.first().map(|p| p.id.clone()).unwrap_or_default()
    });
    let mut query = use_signal(String::new);
    let mut draft = use_signal(String::new);
    let mut draft_image = use_signal(String::new);
    let mut status = use_signal(|| {
        load_error.unwrap_or_else(|| format!("{} prompts", lib().prompts.len()))
    });

    // Seed draft from selection once.
    {
        if draft().is_empty() {
            if let Some(p) = lib().prompts.iter().find(|p| p.id == selected_id()) {
                draft.set(p.prompt.clone());
                draft_image.set(p.image.clone().unwrap_or_default());
            }
        }
    }

    let mut select = move |id: String| {
        selected_id.set(id.clone());
        if let Some(p) = lib().prompts.iter().find(|p| p.id == id) {
            draft.set(p.prompt.clone());
            draft_image.set(p.image.clone().unwrap_or_default());
        }
    };

    let mut save = move |_| {
        let text = draft();
        if text.trim().is_empty() {
            status.set("Write a prompt first".into());
            return;
        }
        let mut l = lib();
        let id = selected_id();
        let img = draft_image().trim().to_string();
        let image = if img.is_empty() { None } else { Some(img) };

        if let Some(idx) = l.prompts.iter().position(|p| p.id == id) {
            l.prompts[idx].prompt = text.clone();
            l.prompts[idx].title = title_from_prompt(&text);
            l.prompts[idx].image = image;
            l.prompts[idx].updated_at = now_iso();
        } else {
            let mut entry = new_prompt_entry(&title_from_prompt(&text), &text);
            entry.image = image;
            selected_id.set(entry.id.clone());
            l.prompts.insert(0, entry);
        }

        match save_library(&l) {
            Ok(()) => {
                let n = l.prompts.len();
                lib.set(l);
                status.set(format!("Saved · {n} prompts"));
            }
            Err(e) => status.set(e),
        }
    };

    let mut new_prompt = move |_| {
        draft.set(String::new());
        draft_image.set(String::new());
        selected_id.set(String::new());
        status.set("New prompt — write, then Save".into());
    };

    let mut copy_prompt = move |_| {
        let text = draft();
        if text.trim().is_empty() {
            status.set("Nothing to copy".into());
            return;
        }
        copy_to_clipboard(&text);
        status.set("Copied".into());
    };

    let q = query();
    let visible: Vec<PromptEntry> = {
        let l = lib();
        if q.trim().is_empty() {
            l.prompts
                .iter()
                .filter(|p| p.storage != "compost")
                .cloned()
                .collect()
        } else {
            let sq = SearchQuery {
                text: q.clone(),
                limit: 200,
                ..Default::default()
            };
            let ids = catalog::search(&sq)
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| catalog::filter_in_memory(&l, &sq));
            l.prompts
                .iter()
                .filter(|p| ids.iter().any(|id| id == &p.id))
                .cloned()
                .collect()
        }
    };

    let current = lib()
        .prompts
        .iter()
        .find(|p| p.id == selected_id() && !selected_id().is_empty())
        .cloned();
    let img_url = current.as_ref().and_then(prompt_image_url);

    rsx! {
        style { {APP_CSS} }
        div { class: "shell",
            aside { class: "library",
                div { class: "lib-head",
                    h1 { "Prompts" }
                    button { class: "btn", onclick: move |_| new_prompt(()), "New" }
                }
                input {
                    class: "search",
                    placeholder: "Search…",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
                nav { class: "list",
                    for p in visible {
                        {
                            let id = p.id.clone();
                            let active = selected_id() == id;
                            let thumb = prompt_image_url(&p);
                            let line = preview(&p.prompt, 80);
                            rsx! {
                                button {
                                    class: if active { "row active" } else { "row" },
                                    onclick: move |_| select(id.clone()),
                                    if let Some(src) = thumb {
                                        img { class: "thumb", src: "{src}", alt: "" }
                                    } else {
                                        div { class: "thumb empty" }
                                    }
                                    span { class: "row-text", "{line}" }
                                }
                            }
                        }
                    }
                }
            }

            main { class: "workspace",
                div { class: "toolbar",
                    button { class: "btn primary", onclick: move |_| copy_prompt(()), "Copy" }
                    button { class: "btn", onclick: move |_| save(()), "Save" }
                    span { class: "status", "{status}" }
                }
                label { class: "hint", "Image prompt (one paragraph)" }
                textarea {
                    class: "prompt",
                    placeholder: "a grey pitbull in the style of Alphonse Mucha, art nouveau poster…",
                    value: "{draft}",
                    oninput: move |e| draft.set(e.value()),
                }
                label { class: "hint", "Result image (optional — file under packs/…/media/)" }
                input {
                    class: "search",
                    placeholder: "my-gen.png  or leave blank",
                    value: "{draft_image}",
                    oninput: move |e| draft_image.set(e.value()),
                }
            }

            aside { class: "image-side",
                h2 { "Result" }
                if let Some(src) = img_url {
                    img { class: "result", src: "{src}", alt: "result" }
                    p { class: "hint", "Image linked to this saved prompt." }
                } else {
                    div { class: "result empty",
                        p { "No image yet." }
                        p { class: "hint",
                            "Save a filename in media/, or drop packs/<deck>/media/<id>.png"
                        }
                    }
                }
            }
        }
    }
}
