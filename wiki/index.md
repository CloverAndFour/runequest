# RuneQuest Internal Wiki

> **CONFIDENTIAL** — This wiki contains hidden mechanics, drop rates, balance formulas, and design rationale.

## Game Wiki

### Core Systems
- [Character Creation & Races](game/characters.md) — 10 races, 10 backgrounds, derived stats, no classes
- [Skill System](game/skills.md) — 44 skills (34 combat + 10 crafting), per-skill XP, ranks 0-10
- [Combat System](game/combat.md) — Weapon-based archetypes, type advantage, actions, damage formulas
- [Progression System](game/progression.md) — Tiers T0-T10, per-skill XP, organizational scaling, permadeath

### Crafting & Equipment
- [Crafting System](game/crafting.md) — 10 crafting skills, gateway staircase, 282 recipes, 336 materials
- [Crafting Stations](game/stations.md) — 12 station types, tier caps, world placement
- [Equipment Lines](game/equipment.md) — 10 equipment lines, weapon/armor stats per tier, crafting requirements
- [Crafting Balance](game/crafting-balance.md) — Graph analysis, complexity metrics, mixing scores, equipment costs
- [Potions & Consumables](game/potions-and-consumables.md) — 70 consumable types, tier scaling, recipes, corruption management, raid logistics

### World & Exploration
- [World Map](game/world.md) — 251K county hex grid, biomes, features, 10 race spawns
- [Monster System](game/monsters.md) — Enemy types, tier stats, drop tables, passive XP
- [Tower System](game/towers.md) — 10 shared infinite dungeons, floor generation, PvP

### Economy & Social
- [Economy](game/economy.md) — Dynamic shops, material pricing, gold sources/sinks
- [Trading & Social](game/trading.md) — Player trading, exchange order book, guild system
- [Balance Calculations](game/balance.md) — Simulator results, stat curves, win rate targets

### Guides
- [New Player Guide](game/newplayer.md) — First 30 minutes walkthrough

## API Reference

- [REST API](api/rest.md) — All HTTP endpoints on port 2998 (supports TLS, JWT + API key auth)
- [WebSocket Protocol](api/websocket.md) — All ClientMsg/ServerMsg types on port 2999 (supports TLS/WSS)
- [Crafting API](api/crafting.md) — Recipe lookup, crafting execution, material queries

**Authentication:** All protected endpoints accept JWT tokens or API keys (`rq_` prefix). API keys are long-lived tokens for programmatic access (AI agents, bots). See the REST API reference for key management endpoints.

**TLS:** All servers support TLS via `--tls-cert` and `--tls-key` CLI args. Frontend auto-detects `wss://` vs `ws://`.
