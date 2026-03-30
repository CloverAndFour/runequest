# Image Generation API Reference

## Overview

Entity images (items, monsters, NPCs) are generated via xAI's Grok Imagine API and cached to disk. Images are served through a public endpoint on both ports.

## Image Serving Endpoint

```
GET /api/images/{category}/{id}
```

**No authentication required.** Available on both port 2998 and port 2999.

### Parameters

| Parameter | Values | Description |
|-----------|--------|-------------|
| `category` | `items`, `monsters`, `npcs` | Entity type |
| `id` | alphanumeric + `_` + `-` | Sanitized entity key |

### Response

- **200**: JPEG image with `Cache-Control: public, max-age=31536000, immutable`
- **400**: Invalid category or id
- **404**: Image not yet generated

### URL Construction

| Entity | URL Pattern | Key Source |
|--------|-------------|------------|
| Known items | `/api/images/items/{item_id}` | Item's `id` field (e.g., `longsword`) |
| Custom items | `/api/images/items/{image_id}` | Item's `image_id` field |
| Monsters | `/api/images/monsters/{name_slug}` | Monster name, lowercased, spaces → `_` |
| NPCs | `/api/images/npcs/{image_id}` | NPC's `image_id` field or sanitized name |

### Examples

```
GET /api/images/items/longsword          → Longsword icon
GET /api/images/monsters/goblin_warrior  → Goblin Warrior portrait
GET /api/images/npcs/marta_the_innkeeper → Marta the Innkeeper portrait
```

## Pre-generation CLI

```bash
cargo run -- generate-images                    # Generate all missing images
cargo run -- generate-images --category items   # Only items
cargo run -- generate-images --category monsters # Only monsters
cargo run -- generate-images --category npcs    # Only fixed NPCs
cargo run -- generate-images --dry-run          # Preview without API calls
cargo run -- generate-images --concurrency 5    # Increase parallelism (default 3)
```

### Entity Counts

| Category | Count | Cost |
|----------|-------|------|
| Items | ~57 | ~$1.14 |
| Monsters | 44 (11 tiers × 4 types) | ~$0.88 |
| Fixed NPCs | 12 | ~$0.24 |
| **Total** | **~113** | **~$2.26** |

## Background Generation

During gameplay, images are generated asynchronously when entities are created:

| Tool Call | Trigger |
|-----------|---------|
| `start_combat` | Generates portraits for all enemies in the encounter |
| `create_npc` | Generates NPC portrait |
| `add_item` | Generates icon for custom (LLM-created) items |

Generation is non-blocking via `tokio::spawn`. The frontend uses `onerror` fallbacks to show emoji/text while images are pending.

## Image Style

All images use the prompt prefix:
> "Detailed fantasy RPG illustration, dark moody background, dramatic lighting. Digital painting style, rich colors, no text or labels."

- **Items**: Centered composition, single object, item icon style
- **Monsters**: Full body visible, menacing, battle-ready
- **NPCs**: Bust portrait, facing viewer, disposition-appropriate expression

## Storage

Images are cached at `{data_dir}/images/{category}/{key}.jpg`:

```
data/images/
  items/longsword.jpg
  items/health_potion.jpg
  monsters/goblin_warrior.jpg
  monsters/lich.jpg
  npcs/marta_the_innkeeper.jpg
```

## Data Model

- `Item.image_id: Option<String>` — Set for LLM-created items; known items use their `id` field
- `Npc.image_id: Option<String>` — Image cache key; fixed NPCs have stable keys

## Security

- IDs are sanitized to `[a-zA-Z0-9_-]` only
- Path traversal (`..`) is rejected with 400
- Images are served with immutable cache headers

## xAI API Details

- **Model**: `grok-imagine-image`
- **Cost**: $0.02 per image
- **Endpoint**: `POST https://api.x.ai/v1/images/generations`
- **Format**: `response_format: "b64_json"`, `aspect_ratio: "1:1"`
- **Output**: JPEG
