# Mor Image Prompt Atelier

A small desktop app for writing **image prompts** — one paragraph of text for Midjourney, Imagine, or similar tools.

- **Main workspace:** edit the prompt, **Copy**, **Save**
- **Open folder…:** native picker chooses where files live
- **Link image…:** optional native picker to attach a result image to that prompt
- **Shelf (PDC):** Dewey-inspired **Prompt Decimal Classification** to file prompts

## Prompt Decimal Classification (PDC)

A small Dewey-style schedule for *image* prompts (not the full library code):

| Code | Shelf |
|------|--------|
| 000 | General & abstract |
| 100 | People & figures |
| 200 | Animals & creatures |
| 300 | Plants & living nature |
| 400 | Places & landscapes |
| 500 | Objects & still life |
| 600 | Architecture & interiors |
| 700 | Arts, styles & media |
| 800 | Scenes, story & action |
| 900 | Mood, light & atmosphere |

Filter the list by hundreds on the left. Assign a code with the class dropdown, or **Suggest** / **Edit → Suggest Classification** (Ctrl+K) from the prompt text. Codes nest by prefix (`700` includes `740`).

Schedule file: [`data/taxonomy.json`](data/taxonomy.json) (override by placing `taxonomy.json` in your workspace folder).

## Storage (mflash folder)

Prompts are stored as a loose **mflash** package in the folder you pick:

```text
your-folder/
  deck.json     # mflash deck (prompt text on each card)
  media/        # optional result images
  library.json  # flat export
  catalog.sqlite  # search index (rebuildable)
```

Each card’s `term` is the image-prompt paragraph. Linked images are copied into `media/` and referenced from the card.

The last folder is remembered in `~/.config/mor-image-prompt-atelier/config.json`.  
Override for scripts/agents: `MOR_PROMPTS_DATA=/path/to/folder`.

Bundled sample prompts ship under `data/` until you open another folder.

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

## MCP (optional)

```bash
node mcp/server.mjs --check
grok mcp add mor-image-prompts -- node "$(pwd)/mcp/server.mjs"
```

See [`mcp/README.md`](mcp/README.md). Point `MOR_PROMPTS_DATA` at the same folder the desktop app uses if you want agents to share files.

## Dev

```bash
cargo test
cargo build --release
```

| Path | Role |
|------|------|
| `src/main.rs` | UI (text + pickers) |
| `src/library.rs` | Workspace, deck.json, image import |
| `src/catalog.rs` | Search index |
| `assets/style.css` | Styles |

## License

See package metadata (`PKGBUILD`).
