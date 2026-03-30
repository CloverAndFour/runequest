# WebSocket Protocol Reference (Port 2999)

## Connection

```
ws://host:2999/ws?token=<JWT>
```

All messages are JSON with a `type` field (snake_case).

## Client -> Server Messages

### Adventure Management

| Type | Fields | Description |
|---|---|---|
| `list_adventures` | -- | Request adventure list |
| `create_adventure` | `name, character_name, race, background?, class?, backstory?` | Create new adventure |
| `load_adventure` | `adventure_id` | Load existing adventure |
| `delete_adventure` | `adventure_id` | Delete adventure |

### Gameplay

| Type | Fields | Description |
|---|---|---|
| `send_message` | `content` | Free-text player input (goes to LLM) |
| `select_choice` | `index, text` | Respond to presented choices |
| `roll_dice` | -- | Execute pending dice roll |
| `get_character_sheet` | -- | Request state update |
| `get_inventory` | -- | Request state update |
| `get_quests` | -- | Request state update |
| `get_npcs` | -- | Request NPC list |
| `set_model` | `model` | Switch LLM model |

### Combat

| Type | Fields | Description |
|---|---|---|
| `combat_action` | `action_id, target?, item_name?` | Submit combat action |

`action_id` values: `attack`, `dodge`, `dash`, `use_item`, `flee`, `second_wind`, `cunning_hide`, `healing_word`, `reckless_attack`, `lay_on_hands`, `flurry_of_blows`, `end_turn`

### Crafting

| Type | Fields | Description |
|---|---|---|
| `craft_item` | `recipe_id` | Craft an item using a recipe |
| `list_recipes` | `skill?, tier?` | List crafting recipes (optional filters) |
| `list_materials` | -- | List all crafting materials |

### Shop

| Type | Fields | Description |
|---|---|---|
| `view_shop` | — | Request shop inventory at current location |
| `shop_buy` | `item_id` (string), `quantity` (u32, default 1) | Buy from shop |
| `shop_sell` | `item_name` (string) | Sell item from inventory |

### Friends

| Type | Fields | Description |
|---|---|---|
| `get_friend_code` | -- | Get your username#code tag |
| `send_friend_request` | `friend_tag` | Send request by tag (e.g. "bob#123456") |
| `accept_friend_request` | `username` | Accept incoming request |
| `decline_friend_request` | `username` | Decline incoming request |
| `remove_friend` | `username` | Remove a friend |
| `get_friends` | -- | Get full friends list with presence |
| `send_chat` | `to, text` | DM a friend |
| `get_chat_history` | `friend, limit?` | Load chat history (default 50) |

### Location Chat

| Type | Fields | Description |
|---|---|---|
| `send_location_chat` | `text` | Send public message at current location |
| `get_location_players` | -- | Request who's at your location |

### Party

| Type | Fields | Description |
|---|---|---|
| `send_party_invite` | `target_username` | Invite player at same location |
| `accept_party_invite` | `from_username` | Accept invite |
| `decline_party_invite` | `from_username` | Decline invite |
| `leave_party` | -- | Leave party |
| `kick_party_member` | `username` | Kick (leader only) |
| `get_party_info` | -- | Get party state |
| `party_combat_action` | `action_id, target?` | Submit combat action in party combat |
| `party_combat_ready` | -- | Ready up (defaults to dodge) |

### PvP

| Type | Fields | Description |
|---|---|---|
| `pvp_challenge` | `target_username` | Challenge player at same location |
| `accept_pvp_challenge` | `challenger` | Accept duel |
| `decline_pvp_challenge` | `challenger` | Decline duel |
| `pvp_action` | `action_id, target?` | Combat action in PvP |
| `pvp_flee` | -- | Flee from duel |

### Trading

| Type | Fields | Description |
|---|---|---|
| `send_trade_request` | `target_username` | Request trade (same location) |
| `accept_trade_request` | `from_username` | Accept trade request |
| `decline_trade_request` | `from_username` | Decline trade request |
| `update_trade_offer` | `items: [{item_name, quantity}], gold` | Set your trade offer |
| `accept_trade` | -- | Accept current trade (both must accept) |
| `cancel_trade` | -- | Cancel active trade |
| `get_trade_status` | -- | Query trade state |

### Exchange

| Type | Fields | Description |
|---|---|---|
| `place_exchange_order` | `item_id, order_type, quantity, price_per_unit` | Place buy/sell order |
| `list_exchange_orders` | `item_id?` | View order book |
| `cancel_exchange_order` | `order_id` | Cancel an order |
| `get_my_exchange_orders` | -- | List your orders |

### Guild

| Type | Fields | Description |
|---|---|---|
| `create_guild` | `name, guild_type` | Create a guild |
| `join_guild` | `guild_name` | Join a guild |
| `leave_guild` | -- | Leave your guild |
| `donate_to_guild` | `gold` | Donate gold to treasury |
| `get_guild_info` | -- | Get guild info |
| `list_guilds` | -- | List all guilds |
| `promote_guild_member` | `username` | Promote a member |
| `kick_guild_member` | `username` | Kick a member |

### Dungeon

| Type | Fields | Description |
|---|---|---|
| `dungeon_enter` | `seed?, tier?` | Enter a dungeon |
| `dungeon_move` | `direction` | Move to adjacent room |
| `dungeon_skill_check` | `direction, skill_id` | Attempt skill gate |
| `dungeon_activate_point` | `puzzle_id, room_id` | Activate puzzle point |
| `dungeon_retreat` | -- | Leave dungeon |
| `dungeon_status` | -- | Get dungeon state |

### Tower

| Type | Fields | Description |
|---|---|---|
| `tower_list` | -- | List all towers |
| `tower_enter` | `tower_id` | Enter a tower |
| `tower_move` | `direction` | Move in tower |
| `tower_ascend` | -- | Go to next floor |
| `tower_checkpoint` | `floor` | Attune checkpoint |
| `tower_teleport` | `target_floor` | Teleport to floor (costs gold) |
| `tower_floor_status` | `tower_id, floor` | Get floor info |

---

## Server -> Client Messages

### Connection

| Type | Fields | Description |
|---|---|---|
| `connected` | `username` | Auth confirmed |

### Adventure Management

| Type | Fields | Description |
|---|---|---|
| `adventure_list` | `adventures` | List of adventures |
| `adventure_loaded` | `state` | Adventure state loaded |
| `adventure_created` | `adventure_id, state` | New adventure created |

### Narrative & Choices

| Type | Fields | Description |
|---|---|---|
| `narrative_chunk` | `text` | Streaming narrative (80-byte chunks, 30ms delay) |
| `narrative_end` | -- | End of narrative stream |
| `present_choices` | `choices, allow_custom_input, prompt` | Choices pending |

### Dice

| Type | Fields | Description |
|---|---|---|
| `dice_roll_request` | `dice_type, count, modifier, dc, description, success_probability` | Roll pending |
| `dice_roll_result` | `rolls, total, dc, success, description` | Roll outcome |

### State

| Type | Fields | Description |
|---|---|---|
| `state_update` | `state` | Full state refresh (includes map_view) |
| `state_changes` | `gold_delta, xp_delta, hp_delta, level_up, items_gained, items_lost, conditions_added, conditions_removed` | State diff after tool loop |

### Cost

| Type | Fields | Description |
|---|---|---|
| `cost_update` | `session_cost_usd, prompt_tokens, completion_tokens, today/week/month/total_cost_usd` | Cost stats |

### Combat (Solo)

| Type | Fields | Description |
|---|---|---|
| `combat_started` | `initiative_order, round` | Combat begins |
| `combat_turn_start` | `combatant, is_player, round, actions, bonus_actions, movement, available_actions, enemies` | Turn start |
| `combat_action_result` | `actor, action, description, roll?, hit?, damage?` | Action outcome |
| `combat_enemy_turn` | `enemy_name, attack_name, attack_roll, target_ac, hit, damage, player_hp, player_max_hp` | Enemy acts |
| `combat_ended` | `xp_reward, victory` | Combat ends |
| `condition_effects` | `effects` | Condition damage at turn start |

### Crafting

| Type | Fields | Description |
|---|---|---|
| `craft_result` | `recipe_name, output, quantity, skill_progress` | Crafting outcome |
| `recipe_list` | `recipes` | List of recipes (with filters applied) |
| `material_list` | `materials` | List of all materials |

### Shop

| Type | Fields | Description |
|---|---|---|
| `shop_inventory` | `shop_name`, `tier`, `items[]` (ShopItemInfo), `player_gold` | Full shop view |
| `shop_buy_result` | `success`, `item_name`, `price`, `gold_remaining`, `error?` | Buy outcome |
| `shop_sell_result` | `success`, `item_name`, `gold_earned`, `gold_remaining`, `error?` | Sell outcome |

On successful buy/sell, the server also sends a `state_update` with the full updated adventure state.

### Friends

| Type | Fields | Description |
|---|---|---|
| `friend_code` | `tag` | Your tag (e.g. "quinten#482193") |
| `friends_list` | `friends[], incoming_requests[], outgoing_requests[]` | Full list with presence |
| `friend_presence` | `username, friend_code, online, character_name?, character_class?, location?` | Real-time status change |
| `friend_request_received` | `from_username, from_tag` | Incoming request notification |
| `friend_request_accepted` | `username, friend_code` | Request accepted |
| `friend_request_sent` | `success, message` | Result of your request |
| `friend_chat` | `from, text, ts` | Incoming chat message |
| `friend_chat_history` | `friend, messages[]` | Chat history |

### Location Chat

| Type | Fields | Description |
|---|---|---|
| `location_chat` | `from, character_name, text, ts, location` | Public chat message |
| `location_presence_update` | `location, players[]` | Who's at this location |
| `location_chat_history` | `location, messages[]` | Recent messages on arrival |

### Party

| Type | Fields | Description |
|---|---|---|
| `party_info` | `party_id, leader, members[], state, location` | Full party state |
| `party_invite_received` | `from_username, from_character, from_class` | Incoming invite |
| `party_invite_sent` | `success, message` | Invite result |
| `party_member_joined` | `username, character_name, character_class` | New member |
| `party_member_left` | `username, reason` | Member departed |
| `party_disbanded` | `reason` | Party dissolved |
| `party_narrative_chunk` | `text` | Shared narrative stream |
| `party_narrative_end` | -- | End of shared narrative |
| `party_combat_started` | `enemies[], initiative_order[], round` | Party combat begins |
| `party_combat_phase_start` | `phase, deadline_ms, round, actions, enemies, party_hp` | 30s timer starts |
| `party_combat_action_ack` | -- | Your action was received |
| `party_combat_action_submitted` | `username` | Someone submitted |
| `party_combat_resolution` | `results[]` | Player actions resolved |
| `party_combat_enemy_phase` | `results[]` | Enemy attacks resolved |
| `party_combat_ended` | `victory, xp_per_member` | Party combat over |
| `party_trap_results` | `results[]` | Per-member trap outcomes |

### PvP

| Type | Fields | Description |
|---|---|---|
| `pvp_challenge_received` | `challenger, character_name` | Incoming challenge |
| `pvp_challenge_sent` | `success, message` | Challenge result |
| `pvp_started` | `opponent_name, opponent_class, opponent_hp, opponent_ac` | Duel begins |
| `pvp_turn_start` | `your_turn, round, opponent_hp, opponent_max_hp, actions` | Turn info |
| `pvp_action_result` | `actor, action, description, roll?, hit?, damage?` | Action outcome |
| `pvp_ended` | `victory, opponent, criminal` | Duel over |
| `criminal_status_update` | `username, is_criminal` | Criminal flag change |

### Trading

| Type | Fields | Description |
|---|---|---|
| `trade_request_received` | `from_username` | Incoming trade request |
| `trade_started` | `partner` | Trade session created |
| `trade_offer_updated` | `your_offer, their_offer` | Offers changed |
| `trade_accepted` | `by` | One player accepted |
| `trade_completed` | `items_gained, items_lost, gold_delta` | Trade executed |
| `trade_cancelled` | `reason` | Trade cancelled |
| `trade_status` | `active, partner?, your_offer?, their_offer?` | Current trade state |

### Exchange

| Type | Fields | Description |
|---|---|---|
| `exchange_order_placed` | `order_id, trades[]` | Order placed |
| `exchange_orders` | `orders[]` | Order book listing |
| `exchange_order_cancelled` | `order_id, refunded` | Order cancelled |

### Guild

| Type | Fields | Description |
|---|---|---|
| `guild_created` | `guild_id, name` | Guild created |
| `guild_info` | `guild` | Full guild data |
| `guild_list` | `guilds[]` | All guilds |
| `guild_member_joined` | `username, guild_name` | New member |
| `guild_member_left` | `username, guild_name` | Member left |

### Dungeon

| Type | Fields | Description |
|---|---|---|
| `dungeon_entered` | `name, tier, floors, room` | Entered dungeon |
| `dungeon_room_changed` | `room, floor, room_id` | Moved to new room |
| `dungeon_skill_gate_result` | `skill, roll, dc, success` | Skill check result |
| `dungeon_puzzle_activation` | `puzzle_id, activated_count, required_count, solved` | Puzzle progress |
| `dungeon_retreated` | `message` | Left dungeon |
| `dungeon_status` | `status` | Current dungeon state |
| `corruption_tick` | `level, effects` | Corruption damage (T7+) |
| `path_cleared` | `path_index, mini_boss` | Split path cleared |
| `convergence_unlocked` | `convergence_room` | All paths cleared |
| `breach_warning` | `message` | Corruption breach warning |

### Tower

| Type | Fields | Description |
|---|---|---|
| `tower_list` | `towers[]` | Available towers |
| `tower_entered` | `tower_name, floor, tier` | Entered tower |
| `tower_floor_status` | `floor` | Floor details |
| `tower_player_nearby` | `player_name, room_x, room_y` | Nearby player |
| `tower_first_clear` | `tower, floor, player` | First clear achievement |

### System

| Type | Fields | Description |
|---|---|---|
| `model_info` | `model, available_models` | Current LLM model info |
| `chat_history` | `entries` | Display history replay on adventure load |
| `error` | `code, message` | Error message |

## Display Event Types

Events stored in display history (replayed on adventure load):

`narrative`, `choice_selected`, `dice_result`, `dice_roll_request`, `choices`, `user_message`, `combat_action`, `combat_enemy`, `combat_started`, `combat_ended`, `state_changes`
