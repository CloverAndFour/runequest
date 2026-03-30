# Crafting API Reference

## REST Endpoints (Port 2998)

### List Recipes

```
GET /api/recipes
GET /api/recipes?skill=smithing
GET /api/recipes?tier=3
GET /api/recipes?skill=smithing&tier=2
```

**Auth**: Not required (reference data)

**Response**:
```json
{
  "recipes": [
    {
      "id": "sm_t2_iron_ingot",
      "name": "Forge Iron Ingot",
      "skill": "Smithing",
      "tier": 2,
      "skill_rank_required": 2,
      "inputs": [
        {"id": "iron_nugget", "name": "Iron Nugget", "quantity": 3},
        {"id": "shaped_wood", "name": "Shaped Wood", "quantity": 1},
        {"id": "sinew_cord", "name": "Sinew Cord", "quantity": 1}
      ],
      "output": "Iron Ingot",
      "output_qty": 1
    }
  ]
}
```

### Get Recipe Details

```
GET /api/recipes/:recipe_id
```

### List Materials

```
GET /api/materials
```

**Response**:
```json
{
  "materials": [
    {"id": "leather_strip", "name": "Leather Strip", "tier": 1},
    {"id": "iron_ingot", "name": "Iron Ingot", "tier": 2}
  ]
}
```

### Craft Item

```
POST /api/adventures/:id/craft
```

**Auth**: Required (Bearer token)

**Request**:
```json
{
  "recipe_id": "sm_t2_iron_ingot"
}
```

**Response** (success):
```json
{
  "result": {
    "crafted": "Forge Iron Ingot",
    "output": "Iron Ingot",
    "quantity": 1,
    "skill_progress": "Smithing improved to Apprentice (2)!"
  },
  "state": { /* full AdventureState */ }
}
```

**Response** (insufficient skill):
```json
{
  "result": {
    "error": "Need Smithing rank 2 (have 1)"
  }
}
```

**Response** (missing materials):
```json
{
  "result": {
    "error": "Need 3x Iron Nugget (have 1)"
  }
}
```

## WebSocket Messages (Port 2999)

### Client → Server

**CraftItem**:
```json
{"type": "CraftItem", "recipe_id": "sm_t2_iron_ingot"}
```

**ListRecipes**:
```json
{"type": "ListRecipes", "skill": "smithing", "tier": 2}
```

**ListMaterials**:
```json
{"type": "ListMaterials"}
```

### Server → Client

**CraftResult**:
```json
{
  "type": "CraftResult",
  "recipe_name": "Forge Iron Ingot",
  "output": "Iron Ingot",
  "quantity": 1,
  "skill_progress": "Smithing improved to Apprentice (2)!"
}
```

**RecipeList**:
```json
{
  "type": "RecipeList",
  "recipes": [...]
}
```

## LLM Tool Definitions

### craft_item
The LLM can craft items on behalf of the player:
```json
{
  "name": "craft_item",
  "description": "Craft an item from a recipe. Requires the right crafting skill rank and materials.",
  "parameters": {
    "recipe_id": {"type": "string", "description": "The recipe ID to craft"}
  },
  "required": ["recipe_id"]
}
```

### list_recipes
```json
{
  "name": "list_recipes",
  "description": "List available crafting recipes. Filter by skill or tier.",
  "parameters": {
    "skill": {"type": "string", "description": "Filter by crafting skill name"},
    "tier": {"type": "integer", "description": "Filter by tier (1-10)"}
  }
}
```

## Crafting Skill Progression

Crafting a recipe has a chance to improve the corresponding crafting skill:

| Recipe Tier vs Current Rank | Improvement Chance |
|---|---|
| At current rank | 15% per craft |
| Above current rank | 25% per craft |
| Below current rank | 0% (no improvement) |

This prevents grinding low-tier recipes for skill advancement.
