# Prompt packs

Each pack is a self-contained shelf of image-prompt ideas (mflash-inspired layout, atelier-native schema).

```text
packs/<pack-id>/
  pack.json           # id, title, tags, license
  prompts/<id>.json   # one PromptEntry each
  media/              # optional reference gens / thumbs
```

## Conventions

| Field | Role |
|-------|------|
| `pack_id` | Shelf slug (must match directory name) |
| `tier` | SS · S · A · B · C craft quality |
| `storage` | hot · cold · compost |
| `subject_class` | character · animal · scene · poster · other |
| `skeleton` | subject / action / setting for JIT re-assembly |
| `fragment_ids` | linked flora atoms |

Desk missions live in `../desk.json` (not inside packs).  
`../catalog.sqlite` is a rebuildable FTS index — do not hand-edit.  
`../library.json` is a flat export for grepping / older tools.
