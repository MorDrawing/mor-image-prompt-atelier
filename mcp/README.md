# mor-image-prompts MCP

stdio MCP server that helps Grok **draft, critique, improve, and store** image-prompt **deck cards** for the Mor Image Prompt Atelier.

Zero dependencies (Node 18+). Data lives under `../data/`:

- `packs/<deck-id>/prompts/*.json`: card content (source of truth)
- `packs/<deck-id>/media/`: reference / result images for cards
- `desk.json`: optional next_experiment + history
- `library.json`: flat export for grepping / older tools
- `styles.json` / `flora.json`: craft DNA

## Atelier loop

1. **Browse / search** decks of cards (desktop app) or `list_library` / `list_packs`.
2. **Draft** subject-first (slots: subject > action > setting > style > medium > composition > lighting).
3. **Ship** via `build_prompt` / `improve_prompt` (flora fills empty lighting/medium, max 2).
4. **Generate** externally (Imagine / etc.): use `prose_variant`, no negatives.
5. **Save** keepers with `save_prompt` — set `image` to a file under `media/` so the card has a face.
6. Optionally **mark** `record_outcome` (`won` | `failed` | `ambiguous`).
7. **Cull** with `storage: cold|compost` when a deck gets noisy.

## Tools

| Tool | Purpose |
|------|---------|
| `get_handbook` | Workflow + craft rules (call first) |
| `list_styles` | Style packs & phrase banks |
| `list_flora` | Micro-prompt flora (weights, pools) |
| `critique_prompt` | Score 0-100, missing slots, recommendations |
| `improve_prompt` | Fill gaps, apply style to improved + prose_variant |
| `build_prompt` | Assemble from slots + flora JIT |
| `vary_prompt` | Style/lighting alternatives |
| `roulette` | Random library core × style pack mashup |
| `list_library` / `list_packs` / `get_prompt` / `save_prompt` / `delete_prompt` | Deck CRUD (+ class filters; cousin warnings; `image` for card face) |
| `mark_copied` | Note a ship to an external generator |
| `record_outcome` | Optional quality mark |
| `get_next_experiment` / `set_next_experiment` | Optional mission continuity |

## Register with Grok

```bash
grok mcp add mor-image-prompts -- node /home/deo-user/ai_workspace/art/mor-image-prompt-atelier/mcp/server.mjs
```

Or in `~/.grok/config.toml`:

```toml
[mcp_servers.mor-image-prompts]
command = "node"
args = ["/home/deo-user/ai_workspace/art/mor-image-prompt-atelier/mcp/server.mjs"]
enabled = true
```

Restart the Grok session (or reload MCP) so tools appear as `mor-image-prompts__*`.

## Self-check

```bash
node mcp/server.mjs --check
```

## Data paths

| Path | Role |
|------|------|
| `data/packs/<id>/` | Deck (`pack.json` + `prompts/*.json` + `media/`) |
| `data/desk.json` | Optional missions |
| `data/library.json` | Flat export (compat) |
| `data/catalog.sqlite` | Rebuildable FTS index (desktop app) |
| `data/styles.json` | Style packs + lexicons |
| `data/flora.json` | Weighted micro-fragments for JIT assembly |

Classification facets: `pack_id` (deck), `tier`, `storage`, `subject_class`, `tags`, `image`.

Override data dir:

```bash
MOR_PROMPTS_DATA=/path/to/data node mcp/server.mjs
```

## Example agent flow

1. `get_handbook`
2. `build_prompt` with `subject` + `style: "pc98"` **or** `improve_prompt` on a rough draft
3. `critique_prompt` on the result
4. `save_prompt` when happy (heed cousin warnings; set `image` after you have a gen)
5. `mark_copied` when shipping to Imagine
6. After the image: optional `record_outcome`; drop the file into `media/<id>.png`
