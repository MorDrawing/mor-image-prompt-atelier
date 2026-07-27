# Mor Image Prompt Atelier

A small desktop app for writing **image prompts** — one paragraph of text, like Midjourney or any other generator.

The main workspace is a text editor. You can **copy** the prompt out, **save** it, and optionally link a **result image** (the image that was generated from that prompt). That’s the whole product.

## Run

```bash
cargo run
# or
cargo build --release && ./target/release/mor_image_prompt_atelier
```

Linux user install:

```bash
cargo build --release
install -Dm755 target/release/mor_image_prompt_atelier ~/.local/bin/mor-image-prompt-atelier
install -Dm644 packaging/mor-image-prompt-atelier.desktop \
  ~/.local/share/applications/mor-image-prompt-atelier.desktop
sed -i "s|Exec=mor-image-prompt-atelier|Exec=$HOME/.local/bin/mor-image-prompt-atelier|" \
  ~/.local/share/applications/mor-image-prompt-atelier.desktop
```

Arch: `makepkg -si` from the project root.

## Layout

```
┌─────────────┬──────────────────────────┬────────────┐
│ Saved list  │  Prompt text (workspace) │ Result img │
│ + search    │  Copy · Save             │ (optional) │
└─────────────┴──────────────────────────┴────────────┘
```

1. Type or paste a prompt.
2. **Copy** → paste into your image generator.
3. **Save** when you want to keep it.
4. Optional: set a filename under that card’s `media/` folder so the result image sits next to the prompt.

## Data

Prompts live as JSON under `data/packs/<deck>/prompts/`. Images go in `data/packs/<deck>/media/`.

| Field | Use |
|-------|-----|
| `prompt` | The paragraph |
| `image` | Filename in `media/` (optional) |
| auto | `media/<id>.png` (etc.) also works |

Override data dir: `MOR_PROMPTS_DATA=/path/to/data`.

More pack layout detail: [`data/packs/README.md`](data/packs/README.md).

## MCP (optional)

Agents can draft/store prompts via [`mcp/`](mcp/README.md):

```bash
node mcp/server.mjs --check
grok mcp add mor-image-prompts -- node "$(pwd)/mcp/server.mjs"
```

## Dev

```bash
cargo test
cargo build --release
```

| Path | Role |
|------|------|
| `src/main.rs` | Desktop UI |
| `src/library.rs` | Load/save prompts + image paths |
| `src/catalog.rs` | Search index |
| `assets/style.css` | UI styles |

## License

See package metadata (`PKGBUILD`).
