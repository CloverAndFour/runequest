/**
 * RuneQuest Unified Action Menu Tests
 * Tests GET /actions and POST /action endpoints.
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
  token = r.data.token;
}

let advId = '';

test.describe('Unified Action Menu', () => {
  test.beforeAll(async () => {
    await login();
    const r = await api('POST', '/api/adventures', {
      name: 'ActionMenuTest', character_name: 'MenuHero', race: 'human', class: 'warrior',
      stats: { strength: 18, dexterity: 14, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
    });
    advId = r.data.state.id;
  });

  test.afterAll(async () => {
    if (advId) await api('DELETE', `/api/adventures/${advId}`);
  });

  test('GET /actions returns action menu with categories', async () => {
    const r = await api('GET', `/api/adventures/${advId}/actions`);
    expect(r.status).toBe(200);
    expect(Array.isArray(r.data.fixed_actions)).toBe(true);
    expect(r.data.fixed_actions.length).toBeGreaterThan(0);

    // Should have gather and work (always available)
    const gather = r.data.fixed_actions.find((a: any) => a.id === 'gather');
    expect(gather).toBeTruthy();
    expect(gather.action).toBe('gather');
    expect(gather.category).toBe('resource');

    const work = r.data.fixed_actions.find((a: any) => a.id === 'work');
    expect(work).toBeTruthy();

    // Should have travel directions with pre-filled params
    const travels = r.data.fixed_actions.filter((a: any) => a.category === 'travel');
    expect(travels.length).toBeGreaterThan(0);
    expect(travels[0].params.direction).toBeTruthy();
    expect(travels[0].action).toBe('travel');

    // LLM actions should be null (lazy)
    expect(r.data.llm_actions).toBeNull();

    // State should be included
    expect(r.data.state).toBeTruthy();
    expect(r.data.state.character).toBeTruthy();
  });

  test('POST /action executes gather and returns updated menu', async () => {
    const r = await api('POST', `/api/adventures/${advId}/action`, {
      action: 'gather', params: {},
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.result.gathered).toBeTruthy();
    expect(r.data.result.biome).toBeTruthy();

    // Response includes updated action menu
    expect(r.data.actions).toBeTruthy();
    expect(r.data.actions.fixed_actions.length).toBeGreaterThan(0);
  });

  test('POST /action executes travel from menu', async () => {
    // Fetch menu
    const menu = await api('GET', `/api/adventures/${advId}/actions`);
    const travel = menu.data.fixed_actions.find((a: any) => a.category === 'travel');
    expect(travel).toBeTruthy();

    // Execute the travel action using action + params from the menu
    const r = await api('POST', `/api/adventures/${advId}/action`, {
      action: travel.action, params: travel.params,
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.result.county_name).toBeTruthy();
  });

  test('POST /action executes work', async () => {
    const r = await api('POST', `/api/adventures/${advId}/action`, {
      action: 'work', params: {},
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.result.gold_earned).toBeGreaterThan(0);
    expect(r.data.result.job).toBeTruthy();
  });

  test('combat actions appear during combat', async () => {
    // Start combat
    await api('POST', `/api/adventures/${advId}/engine/combat`, {
      enemies: [{ name: 'MenuRat', hp: 5, max_hp: 5, ac: 8, enemy_type: 'Brute', tier: 0 }],
    });

    const menu = await api('GET', `/api/adventures/${advId}/actions`);
    expect(menu.data.combat_actions).toBeTruthy();
    expect(menu.data.combat_actions.length).toBeGreaterThan(0);

    // Should have attack with targets
    const attack = menu.data.combat_actions.find((a: any) => a.id === 'attack');
    expect(attack).toBeTruthy();
    expect(attack.targets).toContain('MenuRat');

    // Fixed actions should be disabled or absent during combat
    const gather = menu.data.fixed_actions.find((a: any) => a.id === 'gather');
    // Gather should not be in the list during combat (or disabled)
    if (gather) expect(gather.enabled).toBe(false);
  });

  test('POST /action combat attack works', async () => {
    const r = await api('POST', `/api/adventures/${advId}/action`, {
      action: 'combat', params: { action_id: 'attack', target: 'MenuRat' },
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.result.roll).toBeDefined();
  });

  test('travel blocked during combat via /action', async () => {
    // If still in combat, travel should fail
    const menu = await api('GET', `/api/adventures/${advId}/actions`);
    if (menu.data.combat_actions) {
      const r = await api('POST', `/api/adventures/${advId}/action`, {
        action: 'travel', params: { direction: 'east' },
      });
      expect(r.data.success).toBe(false);
      expect(r.data.error).toContain('combat');
    }
  });

  test('old endpoints still work (backward compat)', async () => {
    // End combat first if active
    const state = await api('GET', `/api/adventures/${advId}`);
    if (state.data.state.combat?.active) {
      // Kill enemy via engine
      for (let i = 0; i < 20; i++) {
        const r = await api('POST', `/api/adventures/${advId}/combat`, {
          action_id: 'attack', target: 'MenuRat',
        });
        if (!r.data.state?.combat?.active) break;
      }
    }

    // Old gather endpoint
    const r = await api('POST', `/api/adventures/${advId}/gather`);
    expect(r.status).toBe(200);
  });

  test('unknown action returns error with menu', async () => {
    const r = await api('POST', `/api/adventures/${advId}/action`, {
      action: 'nonexistent', params: {},
    });
    expect(r.status).toBe(400);
    expect(r.data.success).toBe(false);
    expect(r.data.error).toContain('Unknown action');
    // Even errors should return the action menu
    expect(r.data.actions).toBeTruthy();
  });
});
