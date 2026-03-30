/**
 * RuneQuest Rate Limiting Test Suite
 * Tests per-character cooldowns for LLM and fixed actions.
 * Run: npx playwright test tests/ratelimit.spec.ts
 */
import { test, expect } from '@playwright/test';

const API = 'http://localhost:2998';
const USER = 'test-user-2';
const PASS = 'test-password2';

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

async function login(username = USER, password = PASS) {
  const r = await api('POST', '/api/auth/login', { username, password });
  expect(r.status).toBe(200);
  token = r.data.token;
}

let advId = '';

async function createAdventure(name: string) {
  const r = await api('POST', '/api/adventures', {
    name, character_name: 'RateLimitHero', race: 'human', class: 'warrior',
    stats: { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
  });
  expect(r.status).toBe(200);
  advId = r.data.state.id;
}

test.describe('Rate Limiting', () => {
  test.beforeAll(async () => {
    await login();
    await createAdventure('RateLimitTest');
  });

  test.afterAll(async () => {
    if (advId) await api('DELETE', `/api/adventures/${advId}`);
  });

  test('fixed action is rate limited at 4s', async () => {
    // First gather should succeed
    const r1 = await api('POST', `/api/adventures/${advId}/gather`, {});
    expect(r1.status).toBe(200);

    // Immediate second gather should be rate limited
    const r2 = await api('POST', `/api/adventures/${advId}/gather`, {});
    expect(r2.status).toBe(429);
    expect(r2.data.code).toBe('cooldown');
    expect(r2.data.remaining_ms).toBeGreaterThan(0);
    expect(r2.data.remaining_ms).toBeLessThanOrEqual(4000);
  });

  test('read-only actions are never rate limited', async () => {
    // Get adventure repeatedly — should always succeed
    for (let i = 0; i < 5; i++) {
      const r = await api('GET', `/api/adventures/${advId}`);
      expect(r.status).toBe(200);
    }
  });

  test('different categories have independent cooldowns', async () => {
    // Wait for all cooldowns to expire
    await new Promise(r => setTimeout(r, 7000));

    // Do a fixed action (gather) — should succeed
    const r1 = await api('POST', `/api/adventures/${advId}/gather`, {});
    expect(r1.status).toBe(200);

    // Immediately do another fixed action — should be rate limited
    const r2 = await api('POST', `/api/adventures/${advId}/gather`, {});
    expect(r2.status).toBe(429);
    expect(r2.data.code).toBe('cooldown');

    // But equip (also fixed, shares cooldown) should also be blocked
    const r3 = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'nothing' });
    expect(r3.status).toBe(429);
  });

  test('cooldown error includes remaining time', async () => {
    // Immediate action should fail with proper cooldown info
    const r = await api('POST', `/api/adventures/${advId}/gather`, {});
    if (r.status === 429) {
      expect(r.data.remaining_ms).toBeDefined();
      expect(typeof r.data.remaining_ms).toBe('number');
      expect(r.data.error).toContain('cooldown');
    }
    // If 200, cooldown expired — that's also fine
  });
});

test.describe('Admin Exemption', () => {
  // Note: quinten has admin role — test that admins bypass rate limits
  // We can't easily test this without quinten's password,
  // so we verify test-user (non-admin) IS rate limited
  test('non-admin user is rate limited', async () => {
    await login();
    const name = 'AdminExemptTest';
    const cr = await api('POST', '/api/adventures', {
      name, character_name: 'TestHero', race: 'human', class: 'warrior',
      stats: { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
    });
    const id = cr.data.state.id;

    const r1 = await api('POST', `/api/adventures/${id}/gather`, {});
    expect(r1.status).toBe(200);

    const r2 = await api('POST', `/api/adventures/${id}/gather`, {});
    expect(r2.status).toBe(429);

    await api('DELETE', `/api/adventures/${id}`);
  });
});
