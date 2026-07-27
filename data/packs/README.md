# Decks (prompt packs)

Each **deck** is a shelf of image + prompt cards for the atelier. On disk the folder is still `packs/` (stable path); the UI calls them decks.

```text
packs/<deck-id>/
  pack.json           # id, title, tags, license
  prompts/<id>.json   # one card (PromptEntry) each
  media/              # reference / result images for cards
```

## Card image convention

| Source | How it resolves |
|--------|-----------------|
| `image` field | Filename under `media/` (e.g. `my-gen.png`) |
| `images[]` | Extra refs; first existing file is used as the card thumb |
| Auto | `media/<card-id>.{webp,png,jpg,jpeg,gif,svg}` |

Drop a gen next to the card id and it shows in the deck browser without editing JSON.

## Conventions

| Field | Role |
|-------|------|
| `pack_id` | Deck slug (must match directory name) |
| `image` / `images` | Media filenames for the card face |
| `tier` | SS, S, A, B, C craft quality |
| `storage` | hot, cold, compost |
| `subject_class` | character, animal, scene, poster, other |
| `skeleton` | subject / action / setting for JIT re-assembly |
| `fragment_ids` | linked flora atoms |

Desk missions live in `../desk.json` (optional, not primary UI).  
`../catalog.sqlite` is a rebuildable FTS index. Do not hand-edit.  
`../library.json` is a flat export for grepping / older tools.
