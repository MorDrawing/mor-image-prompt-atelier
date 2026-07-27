//! GTK-style application menu bar, adapted from mor_blogger_theme_editor.

use dioxus::desktop::window;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct MorMenuDropdownProps {
    pub label: String,
    pub children: Element,
}

#[component]
pub fn MorMenuDropdown(props: MorMenuDropdownProps) -> Element {
    rsx! {
        div { class: "mor-menu-item",
            "{props.label}"
            div { class: "mor-menu-dropdown",
                {props.children}
            }
        }
    }
}

#[component]
pub fn MorMenuBar(children: Element) -> Element {
    rsx! {
        nav { class: "mor-menu-bar",
            // System menu: window controls under a brand chip (classic titlebar icon).
            div { class: "mor-menu-item mor-menu-brand",
                span { class: "mor-brand-mark", "M" }
                div { class: "mor-menu-dropdown",
                    MenuItem {
                        label: "Minimize".to_string(),
                        on_action: move |_| window().set_minimized(true)
                    }
                    MenuItem {
                        label: "Maximize / Restore".to_string(),
                        on_action: move |_| window().toggle_maximized()
                    }
                    MenuSeparator {}
                    MenuItem {
                        label: "Close".to_string(),
                        on_action: move |_| { window().close(); }
                    }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn MenuItem(
    label: String,
    #[props(default = None)] shortcut: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] on_action: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        button {
            class: if disabled { "mor-menu-item disabled" } else { "mor-menu-item" },
            onmousedown: move |evt| evt.stop_propagation(),
            onclick: move |e| {
                e.stop_propagation();
                if !disabled {
                    if let Some(h) = on_action { h.call(()); }
                }
            },
            span { "{label}" }
            if let Some(sc) = shortcut {
                span { class: "shortcut", "{sc}" }
            }
        }
    }
}

#[component]
pub fn MenuSeparator() -> Element {
    rsx! { div { class: "mor-menu-divider" } }
}

#[component]
pub fn AppMenuBar(
    on_copy_prompt: EventHandler<()>,
    on_copy_notes: EventHandler<()>,
) -> Element {
    rsx! {
        MorMenuBar {
            MorMenuDropdown { label: "File".to_string(),
                MenuItem {
                    label: "New Prompt".to_string(),
                    disabled: true,
                }
                MenuItem {
                    label: "Import Library…".to_string(),
                    disabled: true,
                }
                MenuItem {
                    label: "Export Library…".to_string(),
                    disabled: true,
                }
                MenuSeparator {}
                MenuItem {
                    label: "Exit".to_string(),
                    shortcut: Some("Ctrl+Q".to_string()),
                    on_action: move |_| { window().close(); }
                }
            }

            MorMenuDropdown { label: "Edit".to_string(),
                MenuItem {
                    label: "Copy Prompt".to_string(),
                    shortcut: Some("Ctrl+C".to_string()),
                    on_action: move |_| on_copy_prompt.call(())
                }
                MenuItem {
                    label: "Copy Notes".to_string(),
                    on_action: move |_| on_copy_notes.call(())
                }
            }

            MorMenuDropdown { label: "View".to_string(),
                MenuItem {
                    label: "Prompt Cabinet".to_string(),
                    disabled: true,
                }
                MenuItem {
                    label: "Image Gallery".to_string(),
                    disabled: true,
                }
            }

            MorMenuDropdown { label: "Help".to_string(),
                MenuItem {
                    label: "About Mor Image Prompt Atelier".to_string(),
                    on_action: move |_| {
                        let _ = dioxus::document::eval(
                            r#"alert("Mor Image Prompt Atelier\nA local-first cabinet for visual incantations.");"#
                        );
                    }
                }
            }
        }
    }
}
