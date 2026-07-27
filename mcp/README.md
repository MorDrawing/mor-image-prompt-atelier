# mor-image-prompts MCP

stdio MCP server that helps Grok **draft, critique, improve, and store** image prompts for the Mor Image Prompt Atelier.

Zero dependencies (Node 18+). Library and style data live under `../data/`.

## Tools

| Tool | Purpose |
|------|---------|
| `get_handbook` | Workflow + craft rules (call first) |
| `list_styles` | Style packs & phrase banks (PC-98, Mucha, …) |
| `critique_prompt` | Score 0–100, missing slots, recommendations |
| `improve_prompt` | Fill gaps, apply style, lean/rich/prose variants |
| `build_prompt` | Assemble from subject/action/setting/… slots |
| `vary_prompt` | Style/lighting alternatives |
| `list_library` / `get_prompt` / `save_prompt` / `delete_prompt` | Local JSON library |

## Register with Grok

```bash
# From this repo (already done if you used the project setup):
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

| File | Role |
|------|------|
| `data/library.json` | Saved prompts |
| `data/styles.json` | Style packs + lexicons |

Override data dir:

```bash
MOR_PROMPTS_DATA=/path/to/data node mcp/server.mjs
```

## Example agent flow

1. `get_handbook`
2. `build_prompt` with `subject` + `style: "pc98"` **or** `improve_prompt` on a rough draft
3. `critique_prompt` on the result
4. `save_prompt` when happy
5. Generate with Imagine using `prose_variant` (2–5 sentences, no negatives)
