# mor-image-prompts MCP

stdio MCP server that helps Grok **draft, critique, improve, scar outcomes, and store** image prompts for the Mor Image Prompt Atelier.

Zero dependencies (Node 18+). Data lives under `../data/`:

- `packs/<pack-id>/prompts/*.json` — prompt content (source of truth)
- `desk.json` — next_experiment + history (missions, not content)
- `library.json` — flat export for grepping / older tools
- `styles.json` / `flora.json` — craft DNA

## Practice loop

1. **Draft** subject-first (slots: subject → action → setting → style → medium → composition → lighting).
2. **Ship** via `build_prompt` / `improve_prompt` (flora fills empty lighting/medium, max 2).
3. **Generate** externally (Imagine / etc.) — use `prose_variant`, no negatives.
4. **Scar** with `record_outcome` (`won` | `failed` | `ambiguous` + note).
5. **Rework** failures (`needs_rework`); inject `last_note` into improve.
6. **Pin** `set_next_experiment` so the desktop **What's next** strip has a mission.
7. **Cull** with `save_prompt` `storage: cold|compost` — keep the hot shelf small.

## Tools

| Tool | Purpose |
|------|---------|
| `get_handbook` | Workflow + craft rules (call first) |
| `list_styles` | Style packs & phrase banks |
| `list_flora` | Micro-prompt flora (weights, pools) |
| `critique_prompt` | Score 0–100, missing slots, recommendations |
| `improve_prompt` | Fill gaps, apply style → improved + prose_variant |
| `build_prompt` | Assemble from slots + flora JIT |
| `vary_prompt` | Style/lighting alternatives |
| `roulette` | Random library core × style pack mashup |
| `list_library` / `list_packs` / `get_prompt` / `save_prompt` / `delete_prompt` | Library CRUD (+ pack/class filters; cousin warnings; shelf via `storage`) |
| `mark_copied` | Note a ship; cold-nag after 2 copies without scar |
| `record_outcome` | Returns desk scar |
| `get_next_experiment` / `set_next_experiment` | Mission continuity (`finish: "done"|"dismissed"`) |

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
| `data/packs/<id>/` | Prompt packs (`pack.json` + `prompts/*.json` + optional `media/`) |
| `data/desk.json` | Missions: next_experiment + experiment_history |
| `data/library.json` | Flat export (compat); not the primary write target |
| `data/catalog.sqlite` | Rebuildable FTS index (desktop app) |
| `data/styles.json` | Style packs + lexicons |
| `data/flora.json` | Weighted micro-fragments for JIT assembly |

Classification facets: `pack_id`, `tier`, `storage`, `subject_class` (character/animal/scene/poster/other), `tags`.

Override data dir:

```bash
MOR_PROMPTS_DATA=/path/to/data node mcp/server.mjs
```

## Example agent flow

1. `get_handbook` / `get_next_experiment`
2. `build_prompt` with `subject` + `style: "pc98"` **or** `improve_prompt` on a rough draft
3. `critique_prompt` on the result
4. `save_prompt` when happy (heed cousin warnings)
5. `mark_copied` when shipping to Imagine
6. After the image: `record_outcome` with a one-line note
7. `set_next_experiment` before ending; `finish: "done"` when finished
