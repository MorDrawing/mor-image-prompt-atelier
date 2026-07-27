//! Custom window chrome (CSD header bar + frameless shell), adapted from
//! mor_blogger_theme_editor's window_frame.

use dioxus::desktop::tao::window::ResizeDirection;
use dioxus::desktop::window;
use dioxus::prelude::*;

#[component]
pub fn MorWindowTitle(title: String, #[props(default = None)] subtitle: Option<String>) -> Element {
    rsx! {
        div { class: "mor-window-title-block",
            span { class: "mor-window-title", "{title}" }
            if let Some(sub) = subtitle {
                span { class: "mor-window-subtitle", "{sub}" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MorHeaderBarProps {
    #[props(default = None)]
    pub start: Option<Element>,
    #[props(default = None)]
    pub center: Option<Element>,
    #[props(default = None)]
    pub end: Option<Element>,
    #[props(default = true)]
    pub show_controls: bool,
}

#[component]
pub fn MorHeaderBar(props: MorHeaderBarProps) -> Element {
    let mut last_click =
        use_signal(|| std::time::Instant::now() - std::time::Duration::from_secs(10));

    let handle_drag = move |_| {
        let now = std::time::Instant::now();
        if now.duration_since(last_click()) < std::time::Duration::from_millis(400) {
            window().toggle_maximized();
            last_click.set(now - std::time::Duration::from_secs(10));
        } else {
            last_click.set(now);
            window().drag();
        }
    };

    rsx! {
        div {
            class: "mor-headerbar",
            onmousedown: handle_drag,

            div { class: "mor-headerbar-start",
                if let Some(s) = props.start { {s} }
            }

            div { class: "mor-headerbar-center",
                if let Some(c) = props.center { {c} }
            }

            div { class: "mor-headerbar-end",
                if let Some(e) = props.end { {e} }

                if props.show_controls {
                    div {
                        class: "mor-window-controls",
                        onmousedown: |e| e.stop_propagation(),
                        button {
                            class: "window-btn",
                            title: "Minimize",
                            onclick: move |_| window().set_minimized(true),
                            "—"
                        }
                        button {
                            class: "window-btn",
                            title: "Maximize",
                            onclick: move |_| window().toggle_maximized(),
                            "□"
                        }
                        button {
                            class: "window-btn close",
                            title: "Close",
                            onclick: move |_| { window().close(); },
                            "×"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MorShell(children: Element) -> Element {
    let is_frameless = std::env::var("MOR_UI_MODE")
        .map(|m| m != "native")
        .unwrap_or(true);

    rsx! {
        div {
            class: "mor-root",

            if is_frameless {
                div { class: "mor-resize-edge top",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::North); }
                }
                div { class: "mor-resize-edge bottom",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::South); }
                }
                div { class: "mor-resize-edge left",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::West); }
                }
                div { class: "mor-resize-edge right",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::East); }
                }
                div { class: "mor-resize-edge top-left",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::NorthWest); }
                }
                div { class: "mor-resize-edge top-right",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::NorthEast); }
                }
                div { class: "mor-resize-edge bottom-left",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::SouthWest); }
                }
                div { class: "mor-resize-edge bottom-right",
                    onmousedown: move |_| { let _ = window().drag_resize_window(ResizeDirection::SouthEast); }
                }
            }

            {children}
        }
    }
}
