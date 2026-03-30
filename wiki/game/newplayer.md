# New Player Guide

## Your First 30 Minutes

### 1. Create Your Character

Choose a **race** (determines your starting location on the map) and a **background** (determines your starting skills and items).

**Recommended first character:**
- **Race:** Human (central spawn, short distances to other races)
- **Background:** Farmhand (starts with Fortitude + Leatherworking, a Spear, and 5 gold)

Why Farmhand? Leatherworking is the T1 gateway skill -- you can start crafting immediately. Fortitude gives you the Second Wind bonus action in combat. The Spear gives you a starting weapon.

### 2. Spawn in Your T0 Village

You arrive at a small village at the edge of the continent. The area is T0 -- very safe. Look around: you'll see a Tanning Rack in the village (every village has one).

### 3. Gather Materials

Use the `gather` action to collect raw materials from the environment. What you get depends on the biome:
- Plains/Forest: Plant Fiber, Wild Herbs, Crude Thread, Green Wood
- Hills/Mountains: Rough Stone, Scrap Metal, Raw Quartz

Gather several times to stock up. Each gather gives 1-3 materials and some Survival XP.

### 4. Kill Rats

T0 monsters are weak: Giant Rats, Cave Spiders, Glow Wisps, Shambling Corpses. They have 3-6 HP and AC 8. Your starting weapon can handle them easily.

Each enemy killed:
- Awards 50 XP (legacy) or passive skill XP (weapon mastery on hit, fortitude on damage taken)
- Has a 60% chance to drop T0 crafting materials (Rat Hide, Spider Silk, etc.)

### 5. Find a Tanning Rack

The Tanning Rack is in your starting village. It supports Leatherworking recipes up to T3.

### 6. Craft Your First Leather Strip

With raw materials from gathering and monster drops, you can craft T1 Leatherworking recipes. The first goal is a **Leather Strip** -- the T1 gateway material.

Check available recipes:
- WebSocket: `list_recipes { skill: "leatherworking" }`
- REST: `GET /api/recipes?skill=leatherworking`

Craft the recipe:
- WebSocket: `craft_item { recipe_id: "lw_t1_leather_strip" }`
- REST: `POST /api/adventures/:id/craft { recipe_id: "lw_t1_leather_strip" }`

Each craft has a 15% chance to improve your Leatherworking skill rank.

### 7. Travel to a T1 Town

Move to an adjacent county. Use the 6 hex directions (East, West, NE, NW, SE, SW). Travel toward higher-tier areas to find T1 towns.

T1 towns have:
- **Basic Forge** (Smithing, up to T3)
- **Woodworking Bench** (Woodworking, up to T3)
- **Loom** (Tailoring, up to T3)
- **Shops** with basic weapons and armor

Watch for travel encounters! T1 counties have a 9% encounter chance.

### 8. Find a Forge and Craft Your First Weapon

At a T1 town with a Basic Forge, you can start Smithing recipes. Your Leather Strips (from step 6) are inputs for T2 Smithing recipes.

The crafting progression: T0 raw materials -> T1 Leatherworking -> T2 Smithing -> T3 Woodworking -> ...

### 9. Equip Your New Gear

Craft equipment from one of the 10 equipment lines:
- **Blade line** (SM+LW+EN): Swords + Heavy Plate
- **Axe line** (SM+LW+WW): Greataxes + Hide Armor
- **Dagger line** (LW+AL+JC): Daggers + Shadow Leather
- **Bow line** (WW+LW+AL): Bows + Ranger Leather

Equip your crafted weapon and armor for a major power boost.

### 10. The Loop Continues

The core gameplay loop:
1. **Gather** raw materials
2. **Kill** monsters for drops and combat XP
3. **Craft** intermediate materials and equipment
4. **Travel** to higher-tier areas for better stations and tougher content
5. **Equip** new gear to handle harder challenges
6. **Repeat** at the next tier

## Tips

- **Death is permanent.** Don't fight monsters way above your tier. If you're at T1, avoid T3+ enemies.
- **Tanning Racks are everywhere.** You can always craft T1 Leatherworking recipes at any village.
- **Shop around.** Shops sell pre-made equipment up to T5 at a 3x markup. Sometimes buying is faster than crafting.
- **Join a party.** Higher-tier content requires teamwork. Use location chat to find other players.
- **Use the exchange.** At exchange locations, you can buy/sell materials on the order book.
- **Multiple skills matter.** Every equipment line requires 3 different crafting skills. Specialize in one line and trade for materials from other skills.

## Key Commands Quick Reference

| Action | WebSocket | REST |
|---|---|---|
| Gather materials | `send_message { content: "gather" }` | LLM-driven |
| View shop | `view_shop` | `GET /api/shop?adventure_id=X` |
| Buy from shop | `shop_buy { item_id, quantity }` | `POST /api/shop/buy` |
| List recipes | `list_recipes { skill?, tier? }` | `GET /api/recipes` |
| Craft item | `craft_item { recipe_id }` | `POST /api/adventures/:id/craft` |
| Equip item | -- | `POST /api/adventures/:id/equip { item_name }` |
| Travel | `send_message { content: "travel east" }` | LLM-driven |
| Combat action | `combat_action { action_id }` | `POST /api/adventures/:id/combat` |
