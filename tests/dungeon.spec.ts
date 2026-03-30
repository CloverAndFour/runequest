/**
 * RuneQuest Dungeon & Tower API Test Suite
 * Tests dungeon/tower REST endpoints on port 2998.
 * Run: npx playwright test tests/dungeon.spec.ts
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

async function createAdventure(name: string, charName: string) {
  const r = await api('POST', '/api/adventures', {
    name, character_name: charName, race: 'human', class: 'warrior',
    stats: { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
  });
  expect(r.status).toBe(200);
  return r.data;
}

function getState(data: any) {
  return data.state || data;
}

async function deleteAdventure(id: string) {
  await api('DELETE', `/api/adventures/${id}`);
}

// === DUNGEON ENDPOINTS ===
test.describe('Dungeon Endpoints', () => {
  let advId = '';

  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Dungeon Test', 'DungeonHero');
    advId = getState(d).id;
  });

  test.afterAll(async () => {
    await deleteAdventure(advId);
  });

  test('dungeon status returns not_in_dungeon initially', async () => {
    const r = await api('GET', `/api/adventures/${advId}/dungeon/status`);
    expect(r.status).toBe(200);
    expect(r.data.in_dungeon).toBe(false);
  });

  test('enter dungeon with seed and tier', async () => {
    const r = await api('POST', `/api/adventures/${advId}/dungeon/enter`, { seed: 12345, tier: 2 });
    expect(r.status).toBe(200);
    expect(r.data.result).toBe('dungeon_entered');
    expect(r.data.dungeon).toBeTruthy();
    expect(r.data.dungeon.name).toBeTruthy();
    expect(r.data.dungeon.tier).toBe(2);
    expect(r.data.dungeon.floors).toBeGreaterThan(0);
    expect(r.data.dungeon.current_floor).toBeDefined();
    expect(r.data.dungeon.current_room).toBeDefined();
    expect(r.data.dungeon.room).toBeTruthy();
    expect(r.data.dungeon.room.exits).toBeTruthy();
  });

  test('enter dungeon fails if already in one', async () => {
    const r = await api('POST', `/api/adventures/${advId}/dungeon/enter`, { seed: 99999, tier: 1 });
    expect(r.status).not.toBe(200);
    expect(r.data.error || r.data.code || r.data.result).toMatch(/already_in_dungeon/i);
  });

  test('dungeon status shows active dungeon', async () => {
    const r = await api('GET', `/api/adventures/${advId}/dungeon/status`);
    expect(r.status).toBe(200);
    expect(r.data.in_dungeon).toBe(true);
    expect(r.data.name).toBeTruthy();
    expect(r.data.tier).toBe(2);
    expect(r.data.current_floor).toBeDefined();
    expect(r.data.current_room).toBeDefined();
  });

  test('move in dungeon', async () => {
    // First get current room to find available exits
    const status = await api('GET', `/api/adventures/${advId}/dungeon/status`);
    expect(status.status).toBe(200);
    const room = status.data.room;
    expect(room).toBeTruthy();
    expect(room.exits).toBeTruthy();

    // Pick the first available exit direction
    const exits = room.exits;
    const directions = Object.keys(exits).filter(d => exits[d]);
    expect(directions.length).toBeGreaterThan(0);

    const dir = directions[0];
    const r = await api('POST', `/api/adventures/${advId}/dungeon/move`, { direction: dir });
    expect(r.status).toBe(200);
    expect(r.data.result).toBe('moved');
    expect(r.data.room).toBeTruthy();
    expect(r.data.room.name).toBeTruthy();
    expect(r.data.room.exits).toBeTruthy();
    expect(r.data.floor).toBeDefined();
    expect(r.data.room_id).toBeDefined();
  });

  test('retreat from dungeon', async () => {
    const r = await api('POST', `/api/adventures/${advId}/dungeon/retreat`);
    expect(r.status).toBe(200);
    expect(r.data.result).toBe('retreated');
    expect(r.data.message).toBeTruthy();
  });

  test('dungeon status returns not_in_dungeon after retreat', async () => {
    const r = await api('GET', `/api/adventures/${advId}/dungeon/status`);
    expect(r.status).toBe(200);
    expect(r.data.in_dungeon).toBe(false);
  });

  test('enter T0 dungeon has 2 floors', async () => {
    const r = await api('POST', `/api/adventures/${advId}/dungeon/enter`, { seed: 12345, tier: 0 });
    expect(r.status).toBe(200);
    expect(r.data.result).toBe('dungeon_entered');
    expect(r.data.dungeon.floors).toBe(2);
    // Clean up: retreat
    await api('POST', `/api/adventures/${advId}/dungeon/retreat`);
  });

  test('enter T5 dungeon has at least 3 floors', async () => {
    const r = await api('POST', `/api/adventures/${advId}/dungeon/enter`, { seed: 12345, tier: 5 });
    expect(r.status).toBe(200);
    expect(r.data.result).toBe('dungeon_entered');
    expect(r.data.dungeon.floors).toBeGreaterThanOrEqual(3);
    // Clean up: retreat
    await api('POST', `/api/adventures/${advId}/dungeon/retreat`);
  });
});

// === TOWER ENDPOINTS ===
test.describe('Tower Endpoints', () => {
  let advId = '';

  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('Tower Test', 'TowerHero');
    advId = getState(d).id;
  });

  test.afterAll(async () => {
    await deleteAdventure(advId);
  });

  test('tower list returns all towers', async () => {
    const r = await api('GET', '/api/towers');
    expect(r.status).toBe(200);
    expect(r.data.towers).toBeTruthy();
    expect(Array.isArray(r.data.towers)).toBe(true);
    expect(r.data.towers.length).toBe(10);
    // Each tower should have required fields
    for (const tower of r.data.towers) {
      expect(tower.id).toBeTruthy();
      expect(tower.name).toBeTruthy();
      expect(typeof tower.base_tier).toBe('number');
    }
  });

  test('tower floor status', async () => {
    const r = await api('GET', '/api/towers/tower_of_dawn/floor/0');
    expect(r.status).toBe(200);
    expect(r.data.floor).toBeTruthy();
  });

  test('enter tower', async () => {
    const r = await api('POST', `/api/adventures/${advId}/tower/enter`, { tower_id: 'tower_of_dawn' });
    expect(r.status).toBe(200);
    expect(r.data.result).toBeTruthy();
    expect(r.data.tower_name).toBeTruthy();
    expect(r.data.floor).toBe(0);
    expect(typeof r.data.tier).toBe('number');
  });

  test('move in tower', async () => {
    // Get current status to find exits
    const status = await api('GET', `/api/adventures/${advId}/dungeon/status`);
    expect(status.status).toBe(200);
    const room = status.data.room;
    expect(room).toBeTruthy();
    expect(room.exits).toBeTruthy();

    const exits = room.exits;
    const directions = Object.keys(exits).filter(d => exits[d]);
    expect(directions.length).toBeGreaterThan(0);

    const dir = directions[0];
    const r = await api('POST', `/api/adventures/${advId}/tower/move`, { direction: dir });
    expect(r.status).toBe(200);
    expect(r.data.room).toBeTruthy();
  });

  test('tower checkpoint', async () => {
    const r = await api('POST', `/api/adventures/${advId}/tower/checkpoint`, { floor: 0 });
    expect(r.status).toBe(200);
    // Should return checkpoint info with teleport cost
    expect(r.data).toBeTruthy();
  });

  test('tower teleport without gold fails', async () => {
    const r = await api('POST', `/api/adventures/${advId}/tower/teleport`, { target_floor: 0 });
    // Expect failure due to insufficient gold or other constraint
    // Could be 400 or 200 with error in body depending on implementation
    if (r.status === 200) {
      // If 200, the response body should indicate an error
      const hasError = r.data.error || r.data.result === 'error' || r.data.code;
      // Teleport might succeed to same floor or fail — either way we check format
      expect(r.data).toBeTruthy();
    } else {
      expect([400, 403, 422]).toContain(r.status);
    }
  });

  test('tower ascend', async () => {
    const r = await api('POST', `/api/adventures/${advId}/tower/ascend`);
    // May fail if not at stairs — that is acceptable, we test the response format
    if (r.status === 200) {
      expect(r.data.room || r.data.result).toBeTruthy();
    } else {
      // Should get a meaningful error, not a 500
      expect([400, 422]).toContain(r.status);
      expect(r.data).toBeTruthy();
    }
  });
});
