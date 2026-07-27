# Mor Image Prompt Atelier

Local desktop atelier for **crafting text prompts for AI image generators**.

Browse **decks** of **image + prompt cards**, search them fast, open a card, refine the text, and copy it into Imagine (or any other generator). Reference gens live next to the prompt so good results become a visual library you can reuse.

![License](https://img.shields.io/badge/license-see%20repo-lightgrey)
![Rust](https://img.shields.io/badge/rust-edition%202024-orange)
![UI](https://img.shields.io/badge/ui-dioxus%20desktop-blue)

## What it is

| Idea | Meaning |
|------|---------|
| **Deck** | A themed shelf of cards (on disk: `data/packs/<id>/`) |
| **Card** | One prompt plus optional face image, tags, notes, tier |
| **Craft** | Side panel to edit, save, and **Copy prompt** |
| **Search** | SQLite FTS over title, tags, prompt text, and class |

Built for iteration: draft → generate externally → drop the image into `media/` → keep the keeper.

## Quick start

**Requirements:** Rust (stable), system libs for Dioxus desktop (GTK3, WebKitGTK 4.1 on Linux).

```bash
git clone https://github.com/MorDrawing/mor-image-prompt-atelier.git
cd mor-image-prompt-atelier
cargo run
```

Release build:

```bash
cargo build --release
./target/release/mor_image_prompt_atelier
```

### Install to your user desktop (Linux)

```bash
cargo build --release
install -Dm755 target/release/mor_image_prompt_atelier ~/.local/bin/mor-image-prompt-atelier
install -Dm644 packaging/mor-image-prompt-atelier.desktop \
  ~/.local/share/applications/mor-image-prompt-atelier.desktop
# Point Exec at the local binary if you prefer an absolute path:
sed -i "s|Exec=mor-image-prompt-atelier|Exec=$HOME/.local/bin/mor-image-prompt-atelier|" \
  ~/.local/share/applications/mor-image-prompt-atelier.desktop

install -Dm644 assets/icons/mor-image-prompt-atelier.svg \
  ~/.local/share/icons/hicolor/scalable/apps/mor-image-prompt-atelier.svg
for size in 16 22 24 32 48 64 128 256 512; do
  install -Dm644 "assets/icons/hicolor/${size}x${size}/apps/mor-image-prompt-atelier.png" \
    "$HOME/.local/share/icons/hicolor/${size}x${size}/apps/mor-image-prompt-atelier.png"
done
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

Arch (from the project root, builds the current tree):

```bash
makepkg -si
```

## Using the atelier

1. **Search** from the top bar (title, tags, prompt body, class).
2. Filter by **deck** or **class** (character, animal, scene, poster, …).
3. Click a **card** → craft panel opens.
4. Edit the prompt, **Copy prompt**, paste into your generator.
5. After a good gen, put the file in that deck’s `media/` folder (see below) so the card has a face.

Optional: mark Won / Failed / Ambiguous on a card if you want a light quality note. There is no mission queue or cold-copy nag in the main UI.

**Roulette** mashes a random library core with a style pack and copies the result for serendipity.

## Data layout

All content is local under `data/` (override with `MOR_PROMPTS_DATA=/path/to/data`).

```text
data/
  packs/<deck-id>/
    pack.json              # deck title, tags, license
    prompts/<card-id>.json # card source of truth
    media/                 # card faces / reference gens
  desk.json                # optional mission sidecar
  library.json             # flat export (compat / grep)
  catalog.sqlite           # rebuildable FTS index (do not hand-edit)
  styles.json              # style packs & phrase banks
  flora.json               # micro-fragments for JIT assembly
```

### Card images

| Source | Resolution |
|--------|------------|
| `image` field | Filename under `packs/<deck>/media/` |
| `images[]` | Extra refs; first existing file wins as thumb |
| Auto | `media/<card-id>.{webp,png,jpg,jpeg,gif,svg}` |

Drop `my-card.png` next to the card id and it shows in the grid without editing JSON. Starter decks ship with simple SVG faces.

Details: [`data/packs/README.md`](data/packs/README.md).

### Starter decks

| Deck | Focus |
|------|--------|
| `murdoch-core` | PC-98 / photocopied manga atelier DNA |
| `characters` | Reusable character archetypes |
| `poster-icons` | Elevated poster subjects |
| `inbox` | New cards land here by default |

## MCP (agent craft)

stdio server for drafting, critiquing, improving, and storing prompts from Grok (or other MCP clients):

```bash
# Self-check
node mcp/server.mjs --check

# Register (example)
grok mcp add mor-image-prompts -- node "$(pwd)/mcp/server.mjs"
```

See [`mcp/README.md`](mcp/README.md) for tools (`build_prompt`, `improve_prompt`, `save_prompt`, `list_library`, …). `save_prompt` accepts `image` / `images` for card faces.

## Development

```bash
cargo test
cargo build --release
```

| Path | Role |
|------|------|
| `src/main.rs` | Dioxus desktop UI (deck browser + craft) |
| `src/library.rs` | Load/save decks, image resolve, flora helpers |
| `src/catalog.rs` | SQLite FTS index |
| `assets/style.css` | Atelier chrome |
| `mcp/server.mjs` | Zero-dep Node MCP server |
| `packaging/` | `.desktop` + icon helpers |
| `PKGBUILD` | Arch package |

Binary name on disk: `mor_image_prompt_atelier`  
Installed / launcher name: `mor-image-prompt-atelier`

## License

See repository / package metadata (`LicenseRef-Proprietary` in `PKGBUILD` unless otherwise noted).
