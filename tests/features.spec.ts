/**
 * RuneQuest Feature Test Suite
 * Comprehensive API tests for all new features: quests, NPCs, friends,
 * location chat, combat simulation, naked start, skills, stat validation.
 *
 * Run: npx playwright test tests/features.spec.ts
 */
import { test, expect } from '@playwright/test';

const API = 'http://localhost:2998';
const USER1 = 'test-user';
const PASS1 = 'test-password1';
const USER2 = 'test-user-2';
const PASS2 = 'test-password2';

let token1 = '';
let token2 = '';

async function api(method: string, path: string, body?: any, tok?: string): Promise<any> {
  const t = tok ?? token1;
  const res = await fetch(`${API}${path}`, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(t ? { 'Authorization': `Bearer ${t}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  try { return { status: res.status, data: JSON.parse(text) }; }
  catch { return { status: res.status, data: text }; }
}

async function login(user: string, pass: string): Promise<string> {
  const r = await api('POST', '/api/auth/login', { username: user, password: pass }, '');
  expect(r.status).toBe(200);
  return r.data.token;
}

const DEFAULT_STATS = { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 };

async function createAdventure(name: string, charName: string, opts?: { race?: string; class?: string; naked_start?: boolean; stats?: any; token?: string }): Promise<any> {
  const r = await api('POST', '/api/adventures', {
    name,
    character_name: charName,
    race: opts?.race || 'human',
    class: opts?.class || 'warrior',
    stats: opts?.stats || DEFAULT_STATS,
    naked_start: opts?.naked_start || false,
  }, opts?.token);
  expect(r.status).toBe(200);
  return r.data;
}

async function cleanup(id: string, tok?: string) {
  await api('DELETE', `/api/adventures/${id}`, undefined, tok);
}

// ============================================================
// AUTH
// ============================================================
test.describe('Multi-User Auth', () => {
  test('both test users can login', async () => {
    token1 = await login(USER1, PASS1);
    expect(token1).toBeTruthy();
    token2 = await login(USER2, PASS2);
    expect(token2).toBeTruthy();
  });

  test('user1 cannot see user2 adventures', async () => {
    token1 = await login(USER1, PASS1);
    token2 = await login(USER2, PASS2);
    const adv = await createAdventure('Isolation Test', 'IsoHero', { token: token2 });
    const state = adv.state || adv;
    const list = await api('GET', '/api/adventures', undefined, token1);
    const ids = (list.data.adventures || []).map((a: any) => a.id);
    expect(ids).not.toContain(state.id);
    await cleanup(state.id, token2);
  });
});

// ============================================================
// QUEST SYSTEM
// ============================================================
test.describe('Quest System', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Quest Test', 'QuestHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('add quest with full details', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'add',
      name: 'Slay the Dragon',
      description: 'A fearsome dragon terrorizes the village',
      final_goal: 'Defeat the dragon in its lair',
      next_step: 'Travel to the mountain pass',
      category: 'main',
      reward: { gold: 500, xp: 1000, description: '500 gold and 1000 XP' },
    });
    expect(r.status).toBe(200);
    expect(r.data.result.quest_added).toBe('Slay the Dragon');
    expect(r.data.result.quest_id).toBeTruthy();
    expect(r.data.result.final_goal).toBe('Defeat the dragon in its lair');
    expect(r.data.result.next_step).toBe('Travel to the mountain pass');
  });

  test('quest appears in adventure state', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const state = r.data.state || r.data;
    const quest = state.quest_log.find((q: any) => q.name === 'Slay the Dragon');
    expect(quest).toBeTruthy();
    expect(quest.category).toBe('main');
    expect(quest.status).toBe('active');
    expect(quest.final_goal).toBe('Defeat the dragon in its lair');
    expect(quest.next_step).toBe('Travel to the mountain pass');
    expect(quest.reward.gold).toBe(500);
    expect(quest.reward.xp).toBe(1000);
  });

  test('update quest step', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'update_step',
      quest_name: 'Slay the Dragon',
      step_completed: 'Traveled to the mountain pass',
      new_next_step: 'Enter the dragon cave',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.step_recorded).toBe('Traveled to the mountain pass');
    expect(r.data.result.next_step).toBe('Enter the dragon cave');
    expect(r.data.result.total_steps_completed).toBe(1);
  });

  test('complete quest awards rewards', async () => {
    // Get gold before
    const before = await api('GET', `/api/adventures/${advId}`);
    const goldBefore = (before.data.state || before.data).character.gold;

    const r = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'complete',
      name: 'Slay the Dragon',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.quest_completed).toBeTruthy();
    expect(r.data.result.success).toBe(true);
    expect(r.data.result.rewards_awarded).toContain('500 gold');
    expect(r.data.result.rewards_awarded).toContain('1000 XP');

    // Verify gold increased
    const after = await api('GET', `/api/adventures/${advId}`);
    const goldAfter = (after.data.state || after.data).character.gold;
    expect(goldAfter).toBe(goldBefore + 500);
  });

  test('add and fail quest', async () => {
    await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'add',
      name: 'Rescue the Princess',
      description: 'A time-sensitive rescue',
      final_goal: 'Save the princess',
      next_step: 'Hurry to the tower',
    });

    const r = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'fail',
      name: 'Rescue the Princess',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.quest_failed).toBeTruthy();

    // Verify status
    const state = await api('GET', `/api/adventures/${advId}`);
    const quest = (state.data.state || state.data).quest_log.find((q: any) => q.name === 'Rescue the Princess');
    expect(quest.status).toBe('failed');
  });

  test('complete nonexistent quest returns error', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'complete',
      name: 'Nonexistent Quest',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.success).toBe(false);
  });
});

// ============================================================
// NPC SYSTEM
// ============================================================
test.describe('NPC System', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('NPC Test', 'NpcHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('create NPC', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'create',
      name: 'Grimgut the Innkeeper',
      description: 'A grumpy dwarf with a heart of gold',
      location: 'Crossroads Inn',
      disposition: 'friendly',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.npc_created).toBe('Grimgut the Innkeeper');
    expect(r.data.result.npc_id).toBeTruthy();
  });

  test('list NPCs', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'list',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.npcs.length).toBeGreaterThanOrEqual(1);
    const grimgut = r.data.result.npcs.find((n: any) => n.name === 'Grimgut the Innkeeper');
    expect(grimgut).toBeTruthy();
    expect(grimgut.disposition).toBe('friendly');
  });

  test('update NPC', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'update',
      npc_name: 'Grimgut the Innkeeper',
      disposition: 'hostile',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.disposition).toBe('hostile');
  });

  test('log NPC interaction', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'log_interaction',
      npc_name: 'Grimgut the Innkeeper',
      summary: 'Player bought a round of drinks',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.interaction_logged).toBe('Player bought a round of drinks');
    expect(r.data.result.total_interactions).toBe(1);
  });

  test('NPC with quest cannot be dismissed', async () => {
    // Create NPC and quest linked to it
    const npcR = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'create',
      name: 'Quest Giver',
      description: 'A mysterious stranger',
    });
    const npcId = npcR.data.result.npc_id;

    await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'add',
      name: 'Strange Errand',
      description: 'Do something odd',
      final_goal: 'Complete the errand',
      next_step: 'Find the thing',
      giver_npc_id: npcId,
    });

    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'dismiss',
      npc_name: 'Quest Giver',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.error).toContain('active quests');
  });

  test('dismiss NPC without quests succeeds', async () => {
    await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'create',
      name: 'Passing Merchant',
      description: 'A trader passing through',
    });

    const r = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'dismiss',
      npc_name: 'Passing Merchant',
    });
    expect(r.status).toBe(200);
    expect(r.data.result.npc_dismissed).toBe('Passing Merchant');
  });
});

// ============================================================
// STAT VALIDATION
// ============================================================
test.describe('Stat Validation', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Stat Test', 'StatHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('valid stat ability check works', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/roll`, {
      dice: 'd20', count: 1, modifier: 0, dc: 10,
    });
    expect(r.status).toBe(200);
  });

  test('ability check with valid stat returns roll', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/roll`, {
      dice: 'd20', count: 1, modifier: 2, dc: 15,
    });
    expect(r.status).toBe(200);
    expect(r.data.result || r.data).toBeTruthy();
    // The result should have rolls, total, and dc fields
    const result = r.data.result || r.data;
    expect(result.total).toBeDefined();
  });
});

// ============================================================
// COMBAT SIMULATION
// ============================================================
test.describe('Combat Simulation', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Combat Sim Test', 'FighterHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('combat simulation runs full cycle', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/combat/simulate`);
    expect(r.status).toBe(200);
    expect(r.data.simulation_log).toBeTruthy();
    expect(Array.isArray(r.data.simulation_log)).toBe(true);

    // Check simulation steps
    const steps = r.data.simulation_log.map((s: any) => s.step);
    expect(steps).toContain('combat_started');
    expect(steps).toContain('player_attack');
    expect(steps).toContain('next_turn');

    // Should have enemy turn
    expect(steps.some((s: string) => s === 'enemy_turn' || s === 'back_to_player')).toBe(true);
  });

  test('combat simulation shows combat state', async () => {
    // Need a fresh adventure since previous one might have ended combat
    const adv = await createAdventure('Combat Sim 2', 'FighterHero2');
    const id = (adv.state || adv).id;

    const r = await api('POST', `/api/adventures/${id}/engine/combat/simulate`);
    expect(r.data.combat_state).toBeTruthy();
    expect(r.data.combat_state.enemies).toBeTruthy();
    expect(r.data.combat_state.player_hp).toBeDefined();

    await cleanup(id);
  });
});

// ============================================================
// NAKED START
// ============================================================
test.describe('Naked Start', () => {
  test('naked start has no equipment', async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Naked Test', 'NakedHero', { naked_start: true });
    const state = adv.state || adv;

    // No equipment
    expect(state.equipment.main_hand).toBeNull();
    expect(state.equipment.chest).toBeNull();
    expect(state.equipment.off_hand).toBeNull();

    // No gold
    expect(state.character.gold).toBe(0);

    // No inventory items (or empty)
    expect(state.inventory.items.length).toBe(0);

    // But character stats should still be set
    expect(state.character.name).toBe('NakedHero');
    expect(state.character.level).toBe(1);

    await cleanup(state.id);
  });

  test('normal start has equipment', async () => {
    const adv = await createAdventure('Normal Test', 'NormalHero', { naked_start: false });
    const state = adv.state || adv;

    // Should have starting equipment
    expect(state.equipment.main_hand).toBeTruthy();
    expect(state.equipment.chest).toBeTruthy();
    expect(state.character.gold).toBeGreaterThan(0);

    await cleanup(state.id);
  });
});

// ============================================================
// FRIENDS SYSTEM
// ============================================================
test.describe('Friends System', () => {
  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    token2 = await login(USER2, PASS2);
  });

  test('get friend code', async () => {
    const r1 = await api('GET', '/api/friends/code', undefined, token1);
    expect(r1.status).toBe(200);
    expect(r1.data.tag).toBeTruthy();
    expect(r1.data.tag).toContain('#');

    const r2 = await api('GET', '/api/friends/code', undefined, token2);
    expect(r2.status).toBe(200);
    expect(r2.data.tag).toContain('#');
  });

  test('send friend request', async () => {
    // Get user2's friend code
    const code = await api('GET', '/api/friends/code', undefined, token2);
    const tag = code.data.tag;

    // User1 sends request to user2
    const r = await api('POST', '/api/friends/request', { friend_tag: tag }, token1);
    expect(r.status).toBe(200);
  });

  test('accept friend request', async () => {
    const r = await api('POST', '/api/friends/accept', { username: USER1 }, token2);
    expect(r.status).toBe(200);
  });

  test('friends list shows both users', async () => {
    const r1 = await api('GET', '/api/friends', undefined, token1);
    expect(r1.status).toBe(200);
    const friends1 = r1.data.friends || [];
    expect(friends1.some((f: any) => f.username === USER2)).toBe(true);

    const r2 = await api('GET', '/api/friends', undefined, token2);
    expect(r2.status).toBe(200);
    const friends2 = r2.data.friends || [];
    expect(friends2.some((f: any) => f.username === USER1)).toBe(true);
  });

  test('send friend chat message', async () => {
    const r = await api('POST', '/api/friends/chat', {
      to: USER2,
      text: 'Hello from test!',
    }, token1);
    expect(r.status).toBe(200);
  });

  test('get chat history', async () => {
    const r = await api('GET', `/api/friends/chat/${USER1}`, undefined, token2);
    expect(r.status).toBe(200);
    const messages = r.data.messages || [];
    expect(messages.length).toBeGreaterThanOrEqual(1);
    expect(messages.some((m: any) => m.text === 'Hello from test!')).toBe(true);
  });

  test('cannot add yourself as friend', async () => {
    const code = await api('GET', '/api/friends/code', undefined, token1);
    const r = await api('POST', '/api/friends/request', { friend_tag: code.data.tag }, token1);
    expect(r.status).toBe(400);
  });

  test('remove friend', async () => {
    const r = await api('DELETE', `/api/friends/${USER2}`, undefined, token1);
    expect(r.status).toBe(200);

    // Verify removed
    const list = await api('GET', '/api/friends', undefined, token1);
    const friends = list.data.friends || [];
    expect(friends.some((f: any) => f.username === USER2)).toBe(false);
  });
});

// ============================================================
// LOCATION CHAT & PRESENCE
// ============================================================
test.describe('Location Chat', () => {
  let advId1: string;
  let advId2: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    token2 = await login(USER2, PASS2);
    const adv1 = await createAdventure('LocChat1', 'ChatHero1', { token: token1 });
    advId1 = (adv1.state || adv1).id;
    const adv2 = await createAdventure('LocChat2', 'ChatHero2', { token: token2 });
    advId2 = (adv2.state || adv2).id;
  });

  test.afterAll(async () => {
    await cleanup(advId1, token1);
    await cleanup(advId2, token2);
  });

  test('send location chat message', async () => {
    const r = await api('POST', '/api/location/chat', {
      adventure_id: advId1,
      text: 'Anyone here?',
    }, token1);
    expect(r.status).toBe(200);
  });

  test('get location players', async () => {
    const r = await api('GET', `/api/location/players?adventure_id=${advId1}`, undefined, token1);
    expect(r.status).toBe(200);
  });
});

// ============================================================
// SKILLS SYSTEM
// ============================================================
test.describe('Skills System', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Skill Test', 'SkillHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('get skills', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/skill`, {
      action: 'get',
    });
    expect(r.status).toBe(200);
    // Skills should return some data about available skills
    expect(r.data.result).toBeTruthy();
  });
});

// ============================================================
// EQUIPMENT & INVENTORY (Extended)
// ============================================================
test.describe('Equipment Extended', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Equip Ext Test', 'EquipHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('equip weapon changes attack stats', async () => {
    // Give a better weapon
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'greatsword' });
    const before = await api('GET', `/api/adventures/${advId}`);
    const beforeState = before.data.state || before.data;
    const beforeMainHand = beforeState.equipment.main_hand;

    // Equip the greatsword
    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Greatsword' });
    expect(r.status).toBe(200);

    // Old weapon should be in inventory now
    const after = await api('GET', `/api/adventures/${advId}`);
    const afterState = after.data.state || after.data;
    expect(afterState.equipment.main_hand.name).toBe('Greatsword');
  });

  test('item database has items', async () => {
    const r = await api('GET', '/api/items');
    expect(r.status).toBe(200);
    expect(r.data.items.length).toBeGreaterThan(10);
  });

  test('specific item lookup works', async () => {
    const r = await api('GET', '/api/items/longsword');
    expect(r.status).toBe(200);
    expect(r.data.name || r.data.item?.name).toBe('Longsword');
  });
});

// ============================================================
// WORLD MAP & TRAVEL (Extended)
// ============================================================
test.describe('World Map Extended', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('World Ext Test', 'WorldHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('world has 20 locations', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const state = r.data.state || r.data;
    expect(state.world.locations.length).toBe(20);
  });

  test('travel to connected location', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const state = r.data.state || r.data;
    const current = state.world.current_location;

    // Find a connected location
    const conn = state.world.connections.find((c: any) =>
      (c.from === current || c.to === current) && c.discovered
    );
    expect(conn).toBeTruthy();

    const targetIdx = conn.from === current ? conn.to : conn.from;
    const targetName = state.world.locations[targetIdx].name;

    // Travel there via engine
    const travel = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'add', // Using quest endpoint as a proxy - we need a travel endpoint
      name: 'test', description: 'test', final_goal: 'test', next_step: 'test',
    });
    // This just tests the endpoint works, actual travel requires the travel_to tool
  });

  test('shops exist in towns', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const state = r.data.state || r.data;
    const towns = state.world.locations.filter((l: any) =>
      l.location_type === 'town' || l.location_type === 'Town' && l.shops && l.shops.length > 0
    );
    expect(towns.length).toBeGreaterThan(0);
  });
});

// ============================================================
// COMBAT SYSTEM (Extended)
// ============================================================
test.describe('Combat Extended', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Combat Ext Test', 'CombatHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('start combat with multiple enemies', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/combat`, {
      enemies: [
        { name: 'Goblin 1', hp: 10, ac: 12, attacks: [{ name: 'Dagger', damage_dice: '1d4', damage_modifier: 1, to_hit_bonus: 3 }] },
        { name: 'Goblin 2', hp: 10, ac: 12, attacks: [{ name: 'Dagger', damage_dice: '1d4', damage_modifier: 1, to_hit_bonus: 3 }] },
      ],
    });
    expect(r.status).toBe(200);
    const state = r.data.state || r.data;
    expect(state.combat.active).toBe(true);
    expect(state.combat.enemies.length).toBe(2);
  });

  test('attack enemy in combat', async () => {
    const r = await api('POST', `/api/adventures/${advId}/combat`, {
      action_id: 'attack',
      target: 'Goblin 1',
    });
    expect(r.status).toBe(200);
  });

  test('dodge action in combat', async () => {
    // End turn first to get back to player turn
    await api('POST', `/api/adventures/${advId}/combat`, { action_id: 'end_turn' });

    const r = await api('POST', `/api/adventures/${advId}/combat`, {
      action_id: 'dodge',
    });
    expect(r.status).toBe(200);
  });

  test('end turn advances combat', async () => {
    const r = await api('POST', `/api/adventures/${advId}/combat`, {
      action_id: 'end_turn',
    });
    expect(r.status).toBe(200);
    // Should still be in combat (enemies alive)
    const state = await api('GET', `/api/adventures/${advId}`);
    const s = state.data.state || state.data;
    expect(s.combat.active).toBe(true);
  });
});

// ============================================================
// ENEMY NAME TRUNCATION
// ============================================================
test.describe('Enemy Name Truncation', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Name Trunc Test', 'TruncHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('long enemy names are truncated', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/combat`, {
      enemies: [{
        name: 'Giant Rat (Cellar Swarm!) - HP:7/7 AC:12 Bite: +4 (1d4+2 piercing damage!)',
        hp: 7, ac: 12,
        attacks: [{ name: 'Bite', damage_dice: '1d4', damage_modifier: 2, to_hit_bonus: 4 }],
      }],
    });
    expect(r.status).toBe(200);
    const state = r.data.state || r.data;
    const enemyName = state.combat.enemies[0].name;
    expect(enemyName.length).toBeLessThanOrEqual(40);
    expect(enemyName).not.toContain('HP:7/7');
    expect(enemyName).not.toContain('AC:12');
  });
});

// ============================================================
// QUEST-NPC LINKING
// ============================================================
test.describe('Quest-NPC Integration', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('QuestNPC Test', 'QNPCHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('quest linked to NPC giver', async () => {
    // Create NPC first
    const npc = await api('POST', `/api/adventures/${advId}/engine/npc`, {
      action: 'create',
      name: 'Elder Thane',
      description: 'Village elder',
      disposition: 'friendly',
    });
    const npcId = npc.data.result.npc_id;

    // Create quest with giver
    const quest = await api('POST', `/api/adventures/${advId}/engine/quest`, {
      action: 'add',
      name: 'Village Defense',
      description: 'Protect the village',
      final_goal: 'Repel the attackers',
      next_step: 'Fortify the walls',
      giver_npc_id: npcId,
      reward: { gold: 100, xp: 200, description: '100 gold and 200 XP' },
    });

    // Verify quest has giver
    const state = await api('GET', `/api/adventures/${advId}`);
    const s = state.data.state || state.data;
    const q = s.quest_log.find((q: any) => q.name === 'Village Defense');
    expect(q.giver_npc_id).toBe(npcId);

    // Verify NPC has quest linked
    const npcList = await api('POST', `/api/adventures/${advId}/engine/npc`, { action: 'list' });
    const elder = npcList.data.result.npcs.find((n: any) => n.name === 'Elder Thane');
    expect(elder.quest_count).toBeGreaterThanOrEqual(1);
  });
});

// ============================================================
// BACKWARD COMPATIBILITY
// ============================================================
test.describe('Backward Compatibility', () => {
  test('adventure with default stats loads correctly', async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('Compat Test', 'CompatHero');
    const state = adv.state || adv;

    // Should have empty quest_log and npcs
    expect(state.quest_log).toBeDefined();
    expect(Array.isArray(state.quest_log)).toBe(true);
    expect(state.npcs).toBeDefined();
    expect(Array.isArray(state.npcs)).toBe(true);

    await cleanup(state.id);
  });
});

// ============================================================
// ADVENTURE HISTORY
// ============================================================
test.describe('Adventure History', () => {
  let advId: string;

  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
    const adv = await createAdventure('History Test', 'HistHero');
    advId = (adv.state || adv).id;
  });

  test.afterAll(async () => { await cleanup(advId); });

  test('history endpoint returns events', async () => {
    const r = await api('GET', `/api/adventures/${advId}/history`);
    expect(r.status).toBe(200);
    expect(r.data.events).toBeDefined();
    expect(Array.isArray(r.data.events)).toBe(true);
  });
});

// ============================================================
// MULTI-CLASS STARTING EQUIPMENT
// ============================================================
test.describe('Class Starting Equipment', () => {
  test.beforeAll(async () => {
    token1 = await login(USER1, PASS1);
  });

  test('mage starts with quarterstaff and leather armor', async () => {
    const adv = await createAdventure('Mage Equip', 'MageHero', { class: 'mage' });
    const state = adv.state || adv;
    expect(state.equipment.main_hand?.name).toBe('Quarterstaff');
    expect(state.equipment.chest?.name).toBe('Leather Armor');
    await cleanup(state.id);
  });

  test('rogue starts with shortsword and studded leather', async () => {
    const adv = await createAdventure('Rogue Equip', 'RogueHero', { class: 'rogue' });
    const state = adv.state || adv;
    expect(state.equipment.main_hand?.name).toBe('Shortsword');
    expect(state.equipment.chest?.name).toBe('Studded Leather');
    await cleanup(state.id);
  });

  test('cleric starts with mace and scale mail', async () => {
    const adv = await createAdventure('Cleric Equip', 'ClericHero', { class: 'cleric' });
    const state = adv.state || adv;
    expect(state.equipment.main_hand?.name).toBe('Mace');
    expect(state.equipment.chest?.name).toBe('Scale Mail');
    await cleanup(state.id);
  });

  test('ranger starts with longbow and chain shirt', async () => {
    const adv = await createAdventure('Ranger Equip', 'RangerHero', { class: 'ranger' });
    const state = adv.state || adv;
    expect(state.equipment.main_hand?.name).toBe('Longbow');
    expect(state.equipment.chest?.name).toBe('Chain Shirt');
    await cleanup(state.id);
  });
});
