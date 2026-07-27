mod menu_bar;
mod window_frame;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

use menu_bar::AppMenuBar;
use window_frame::{MorHeaderBar, MorShell, MorWindowTitle};

// Embed CSS so the desktop binary works when launched outside `cargo run`
// (installed packages don't set CARGO_MANIFEST_DIR, so asset! paths fail).
static APP_CSS: &str = include_str!("../assets/style.css");

fn ui_mode() -> String {
    std::env::var("MOR_UI_MODE").unwrap_or_else(|_| "frameless".to_string())
}

fn main() {
    let is_native = ui_mode() == "native";

    let cfg = Config::new()
        .with_menu(None::<dioxus::desktop::muda::Menu>)
        .with_window(
            WindowBuilder::new()
                .with_title("Mor Image Prompt Atelier")
                .with_inner_size(LogicalSize::new(1280.0, 800.0))
                .with_decorations(is_native)
                .with_transparent(!is_native),
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

#[derive(Clone, PartialEq)]
struct PromptCard {
    title: &'static str,
    tier: &'static str,
    tags: &'static [&'static str],
    prompt: &'static str,
    notes: &'static str,
}

const STARTER_PROMPTS: &[PromptCard] = &[
    PromptCard {
        title: "PC98 Dark Academia Wordsmiths",
        tier: "SS",
        tags: &["pc98", "dark-academia", "atelier", "anime", "wordsmith"],
        prompt: "stippled shadows, atelier with exactly three anime wordsmiths gathered around a typewriter, elegant loose-tie man, dark Gibson-girl with corset, reserved scholar with glasses, photocopied manga texture, black background, delicate white linework",
        notes: "Core Murdoch image-prompt flavor. Good for landscape banner experiments.",
    },
    PromptCard {
        title: "Perpending Professor",
        tier: "S",
        tags: &["professor", "noir", "scholarly", "danger"],
        prompt: "somberly-dressed perpending professor, brooding at night, foggy cobblestone street, dark academia, faint whiff of danger, pen and ink, delicate linework",
        notes: "Useful as a reusable character archetype.",
    },
    PromptCard {
        title: "Mucha Pitbull",
        tier: "A",
        tags: &["mucha", "art-nouveau", "animal", "poster"],
        prompt: "a grey pitbull in the style of Alphonse Mucha, art nouveau poster composition, ornate frame, elegant linework, dignified expression",
        notes: "Good example of turning a funny subject into an elevated poster.",
    },
];

#[component]
fn App() -> Element {
    let mut selected = use_signal(|| 0usize);
    let current = &STARTER_PROMPTS[selected()];
    let is_native = ui_mode() == "native";

    let prompt_text = current.prompt;
    let notes_text = current.notes;

    rsx! {
        style { {APP_CSS} }

        MorShell {
            if !is_native {
                MorHeaderBar {
                    show_controls: true,
                    start: rsx! { div { style: "width: 16px;" } },
                    center: rsx! {
                        MorWindowTitle {
                            title: "Mor Image Prompt Atelier".to_string(),
                            subtitle: Some("local prompt cabinet".to_string())
                        }
                    },
                    end: rsx! { div { style: "width: 16px;" } }
                }
            }

            div { class: "editor-shell",
                AppMenuBar {
                    on_copy_prompt: move |_| copy_to_clipboard(prompt_text),
                    on_copy_notes: move |_| copy_to_clipboard(notes_text),
                }

                main { class: "app-shell",
                    aside { class: "sidebar",
                        h1 { "Prompt Cabinet" }
                        p { class: "subtitle", "A local-first cabinet for visual incantations." }

                        nav { class: "prompt-list",
                            for (index, prompt) in STARTER_PROMPTS.iter().enumerate() {
                                button {
                                    class: if selected() == index { "prompt-button active" } else { "prompt-button" },
                                    onclick: move |_| selected.set(index),
                                    span { class: "prompt-title", "{prompt.title}" }
                                    span { class: "prompt-tier", "Tier {prompt.tier}" }
                                }
                            }
                        }
                    }

                    section { class: "editor-panel",
                        div { class: "panel-header",
                            div {
                                h2 { "{current.title}" }
                                p { "Tier {current.tier}" }
                            }
                            button {
                                class: "copy-button",
                                onclick: move |_| copy_to_clipboard(prompt_text),
                                "Copy Prompt"
                            }
                        }

                        div { class: "tag-row",
                            for tag in current.tags {
                                span { class: "tag", "#{tag}" }
                            }
                        }

                        label { class: "field-label", "Prompt" }
                        textarea {
                            class: "prompt-textarea",
                            value: "{current.prompt}",
                            readonly: true,
                        }

                        label { class: "field-label", "Notes" }
                        textarea {
                            class: "notes-textarea",
                            value: "{current.notes}",
                            readonly: true,
                        }
                    }

                    aside { class: "image-panel",
                        h2 { "Image Gallery" }
                        div { class: "image-placeholder",
                            p { "No images attached yet." }
                            small { "Later: drag PNGs/JPGs here and link them to prompts." }
                        }

                        div { class: "future-box",
                            h3 { "Planned" }
                            ul {
                                li { "SQLite prompt library" }
                                li { "Local image attachments" }
                                li { "Prompt variation history" }
                                li { "Blogger import/export" }
                            }
                        }
                    }
                }
            }
        }
    }
}
