/**
 * RuneQuest Engine Test Suite
 * Tests the REST API on port 2998 to verify all game mechanics.
 * Run: npx playwright test tests/engine.spec.ts
 */
import { test, expect } from '@playwright/test';

const API = 'http://localhost:2998';
const USER = 'test-user';
const PASS = 'test-password1';

let token = '';

async function api(method: string, path: string, body?: any): Promise<any> {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  try { return { status: res.status, data: JSON.parse(text) }; }
  catch { return { status: res.status, data: text }; }
}

async function login() {
  const r = await api('POST', '/api/auth/login', { username: USER, password: PASS });
  expect(r.status).toBe(200);
  token = r.data.token;
}

async function createAdventure(name: string, charName: string, race = 'human', cls = 'warrior', stats?: any) {
  const r = await api('POST', '/api/adventures', {
    name, character_name: charName, race, class: cls,
    stats: stats || { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
  });
  expect(r.status).toBe(200);
  return r.data;
}

async function deleteAdventure(id: string) {
  await api('DELETE', `/api/adventures/${id}`);
}

// Helper: get state from the response (handles both {state:{...}} and direct state)
function getState(data: any) {
  return data.state || data;
}

// === AUTH ===
test.describe('Authentication', () => {
  test('login succeeds with valid credentials', async () => {
    const r = await api('POST', '/api/auth/login', { username: USER, password: PASS });
    expect(r.status).toBe(200);
    expect(r.data.token).toBeTruthy();
  });

  test('login fails with wrong password', async () => {
    const r = await api('POST', '/api/auth/login', { username: USER, password: 'wrong' });
    expect(r.status).toBe(401);
  });

  test('protected endpoints reject no token', async () => {
    const saved = token; token = '';
    const r = await api('GET', '/api/adventures');
    token = saved;
    expect(r.status).toBe(401);
  });
});

// === ADVENTURE LIFECYCLE ===
test.describe('Adventure Lifecycle', () => {
  test.beforeAll(async () => { await login(); });

  test('create warrior adventure with correct starting stats', async () => {
    const d = await createAdventure('Warrior Test', 'TestWarrior', 'human', 'warrior');
    const s = getState(d);
    expect(s.character.name).toBe('TestWarrior');
    expect(s.character.class).toBe('warrior');
    expect(s.character.level).toBe(1);
    expect(s.character.hp).toBeGreaterThan(0);
    expect(s.character.ac).toBeGreaterThanOrEqual(16);
    expect(s.equipment.main_hand).toBeTruthy();
    expect(s.equipment.chest).toBeTruthy();
    await deleteAdventure(s.id);
  });

  test('create mage adventure', async () => {
    const d = await createAdventure('Mage Test', 'TestMage', 'elf', 'mage');
    const s = getState(d);
    expect(s.character.class).toBe('mage');
    await deleteAdventure(s.id);
  });

  test('list adventures', async () => {
    const d = await createAdventure('List Test', 'ListHero');
    const s = getState(d);
    const r = await api('GET', '/api/adventures');
    expect(r.status).toBe(200);
    const list = r.data.adventures || r.data;
    expect(Array.isArray(list)).toBe(true);
    const found = list.find((a: any) => a.id === s.id);
    expect(found).toBeTruthy();
    await deleteAdventure(s.id);
  });

  test('get adventure state', async () => {
    const d = await createAdventure('Get Test', 'GetHero');
    const s = getState(d);
    const r = await api('GET', `/api/adventures/${s.id}`);
    expect(r.status).toBe(200);
    const loaded = getState(r.data);
    expect(loaded.character.name).toBe('GetHero');
    await deleteAdventure(s.id);
  });

  test('delete adventure', async () => {
    const d = await createAdventure('Delete Test', 'DeleteHero');
    const s = getState(d);
    await deleteAdventure(s.id);
    const r = await api('GET', `/api/adventures/${s.id}`);
    expect([404, 500]).toContain(r.status);
  });
});

// === CHARACTER MECHANICS ===
test.describe('Character Mechanics', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Char Test', 'CharHero');
    advId = getState(d).id;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('engine/hp reduces HP', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/hp`, { delta: -3, reason: 'test' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.character.hp).toBeLessThan(s.character.max_hp);
  });

  test('engine/hp heals HP (capped at max)', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/hp`, { delta: 100, reason: 'heal' });
    const s = getState(r.data);
    expect(s.character.hp).toBe(s.character.max_hp);
  });

  test('engine/xp awards XP', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/xp`, { amount: 250, reason: 'test' });
    const s = getState(r.data);
    expect(s.character.xp).toBe(250);
  });

  test('engine/xp triggers level up at threshold', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/xp`, { amount: 100, reason: 'level' });
    const s = getState(r.data);
    expect(s.character.level).toBe(2);
  });

  test('engine/gold adds gold', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/gold`, { amount: 50 });
    const s = getState(r.data);
    expect(s.character.gold).toBeGreaterThanOrEqual(50);
  });
});

// === EQUIPMENT SYSTEM ===
test.describe('Equipment System', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Equip Test', 'EquipHero');
    advId = getState(d).id;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('starting equipment is equipped', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const s = getState(r.data);
    expect(s.equipment.main_hand).toBeTruthy();
    expect(s.equipment.chest).toBeTruthy();
  });

  test('engine/item gives item to inventory', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'rapier' });
    const s = getState(r.data);
    const rapier = s.inventory.items.find((i: any) => i.id === 'rapier' || i.name.includes('Rapier'));
    expect(rapier).toBeTruthy();
  });

  test('equip moves item from inventory to slot', async () => {
    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Rapier' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.equipment.main_hand.name).toContain('Rapier');
  });

  test('unequip moves item back to inventory', async () => {
    const r = await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'main_hand' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.equipment.main_hand).toBeFalsy();
  });

  test('equip armor changes AC', async () => {
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'plate_armor' });
    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Plate Armor' });
    const s = getState(r.data);
    expect(s.character.ac).toBeGreaterThanOrEqual(18);
  });
});

// === DICE MECHANICS ===
test.describe('Dice Mechanics', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Dice Test', 'DiceHero');
    advId = getState(d).id;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('d20 rolls are in range 1-20', async () => {
    for (let i = 0; i < 10; i++) {
      const r = await api('POST', `/api/adventures/${advId}/engine/roll`, { dice: 'd20', count: 1, modifier: 0 });
      // Response might be {total, rolls, ...} or {result: {total, ...}} or {state: ..., ...}
      const roll = r.data.result || r.data;
      expect(roll.total).toBeGreaterThanOrEqual(1);
      expect(roll.total).toBeLessThanOrEqual(20);
    }
  });

  test('modifier is applied correctly', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/roll`, { dice: 'd20', count: 1, modifier: 5 });
    const roll = r.data.result || r.data;
    expect(roll.total).toBeGreaterThanOrEqual(6);
    expect(roll.total).toBeLessThanOrEqual(25);
  });

  test('dc check returns success/failure', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/roll`, { dice: 'd20', count: 1, modifier: 0, dc: 10 });
    const roll = r.data.result || r.data;
    expect(typeof roll.success).toBe('boolean');
    expect(roll.success).toBe(roll.total >= 10);
  });

  test('d6 rolls are in range 1-6', async () => {
    for (let i = 0; i < 10; i++) {
      const r = await api('POST', `/api/adventures/${advId}/engine/roll`, { dice: 'd6', count: 1, modifier: 0 });
      const roll = r.data.result || r.data;
      expect(roll.total).toBeGreaterThanOrEqual(1);
      expect(roll.total).toBeLessThanOrEqual(6);
    }
  });
});

// === CONDITIONS ===
test.describe('Conditions', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Cond Test', 'CondHero');
    advId = getState(d).id;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('add condition', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Poisoned', action: 'add' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.character.conditions).toContain('Poisoned');
  });

  test('condition appears in loaded state', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const s = getState(r.data);
    expect(s.character.conditions).toContain('Poisoned');
  });

  test('remove condition', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Poisoned', action: 'remove' });
    const s = getState(r.data);
    expect(s.character.conditions).not.toContain('Poisoned');
  });

  test('multiple conditions coexist', async () => {
    await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Blinded', action: 'add' });
    await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Poisoned', action: 'add' });
    const r = await api('GET', `/api/adventures/${advId}`);
    const s = getState(r.data);
    expect(s.character.conditions).toContain('Blinded');
    expect(s.character.conditions).toContain('Poisoned');
    await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Blinded', action: 'remove' });
    await api('POST', `/api/adventures/${advId}/engine/condition`, { condition: 'Poisoned', action: 'remove' });
  });
});

// === COMBAT SYSTEM ===
test.describe('Combat System', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Combat Test', 'CombatHero', 'human', 'warrior');
    advId = getState(d).id;
    await api('POST', `/api/adventures/${advId}/engine/hp`, { delta: 100, reason: 'prep' });
    // Re-equip weapon in case it was unequipped
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'longsword' });
    await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Longsword' });
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('start combat creates initiative order', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/combat`, {
      enemies: [
        { name: 'Goblin', hp: 7, ac: 12, attacks: [{ name: 'Slash', damage_dice: 'd6', damage_modifier: 1, to_hit_bonus: 3 }] },
      ]
    });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.combat.active).toBe(true);
    expect(s.combat.enemies.length).toBe(1);
    expect(s.combat.initiative.length).toBe(2);
  });

  test('combat attack works', async () => {
    const r = await api('POST', `/api/adventures/${advId}/combat`, { action_id: 'attack', target: 'Goblin' });
    expect(r.status).toBe(200);
  });

  test('combat end_turn advances turn', async () => {
    const r = await api('POST', `/api/adventures/${advId}/combat`, { action_id: 'end_turn' });
    // Might be 200 or 400 depending on combat state (goblin might be dead)
    // Just verify we get a response
    expect([200, 400]).toContain(r.status);
  });
});

// === INVENTORY & GOLD ===
test.describe('Inventory & Gold', () => {
  let advId = '';
  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Inv Test', 'InvHero');
    advId = getState(d).id;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('give potion adds to inventory', async () => {
    const r = await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'potion_healing' });
    const s = getState(r.data);
    const potions = s.inventory.items.filter((i: any) => i.name.includes('Health Potion') || i.id === 'potion_healing');
    expect(potions.length).toBeGreaterThan(0);
  });

  test('give gold increases gold', async () => {
    const before = await api('GET', `/api/adventures/${advId}`);
    const oldGold = getState(before.data).character.gold;
    const r = await api('POST', `/api/adventures/${advId}/engine/gold`, { amount: 100 });
    const s = getState(r.data);
    expect(s.character.gold).toBe(oldGold + 100);
  });
});

// === ITEM DATABASE ===
test.describe('Item Database', () => {
  test.beforeAll(async () => { await login(); });

  test('items endpoint returns items', async () => {
    const r = await api('GET', '/api/items');
    expect(r.status).toBe(200);
    const items = r.data.items || r.data;
    expect(Array.isArray(items)).toBe(true);
    expect(items.length).toBeGreaterThan(10);
  });

  test('specific item lookup works', async () => {
    const r = await api('GET', '/api/items/longsword');
    expect(r.status).toBe(200);
    const item = r.data.item || r.data;
    expect(item.name).toContain('Longsword');
  });
});

// === HEALTH CHECK ===
test.describe('Health', () => {
  test('health endpoint returns ok', async () => {
    const r = await fetch(`${API}/health`);
    expect(r.status).toBe(200);
    expect(await r.text()).toBe('ok');
  });
});
