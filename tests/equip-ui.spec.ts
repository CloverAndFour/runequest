/**
 * RuneQuest Equip/Unequip API Tests
 * Tests the REST API equip/unequip endpoints that the UI calls.
 * Run: npx playwright test tests/equip-ui.spec.ts
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

function getState(data: any) {
  return data.state || data;
}

// === EQUIP/UNEQUIP UI API ===
test.describe('Equip/Unequip API (UI-driven)', () => {
  let advId = '';
  let initialAc = 0;

  test.beforeAll(async () => {
    await login();
    const d = await createAdventure('EquipUI Test', 'EquipUIHero');
    const s = getState(d);
    advId = s.id;
    initialAc = s.character.ac;
  });
  test.afterAll(async () => { await deleteAdventure(advId); });

  test('warrior starts with main_hand and chest equipped', async () => {
    const r = await api('GET', `/api/adventures/${advId}`);
    const s = getState(r.data);
    expect(s.equipment.main_hand).toBeTruthy();
    expect(s.equipment.main_hand.name).toBeTruthy();
    expect(s.equipment.chest).toBeTruthy();
    expect(s.equipment.chest.name).toBeTruthy();
  });

  test('unequip main_hand moves weapon to inventory', async () => {
    // Get current weapon name
    const before = await api('GET', `/api/adventures/${advId}`);
    const weaponName = getState(before.data).equipment.main_hand.name;

    const r = await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'main_hand' });
    expect(r.status).toBe(200);
    const s = getState(r.data);

    // Slot should be empty
    expect(s.equipment.main_hand).toBeFalsy();

    // Weapon should be in inventory
    const inInventory = s.inventory.items.find((i: any) => i.name === weaponName);
    expect(inInventory).toBeTruthy();
  });

  test('equip weapon from inventory fills main_hand slot', async () => {
    // Find the weapon we just unequipped
    const before = await api('GET', `/api/adventures/${advId}`);
    const weapon = getState(before.data).inventory.items.find((i: any) =>
      i.item_type === 'weapon' || i.slot === 'main_hand'
    );
    expect(weapon).toBeTruthy();

    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: weapon.name });
    expect(r.status).toBe(200);
    const s = getState(r.data);

    // Weapon should be equipped
    expect(s.equipment.main_hand).toBeTruthy();
    expect(s.equipment.main_hand.name).toBe(weapon.name);

    // Weapon should NOT be in inventory
    const stillInInv = s.inventory.items.find((i: any) => i.name === weapon.name);
    expect(stillInInv).toBeFalsy();
  });

  test('unequip chest armor lowers AC', async () => {
    const before = await api('GET', `/api/adventures/${advId}`);
    const acBefore = getState(before.data).character.ac;

    const r = await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'chest' });
    expect(r.status).toBe(200);
    const s = getState(r.data);

    expect(s.character.ac).toBeLessThan(acBefore);
    expect(s.equipment.chest).toBeFalsy();
  });

  test('equip armor from inventory raises AC', async () => {
    const before = await api('GET', `/api/adventures/${advId}`);
    const sBefore = getState(before.data);
    const acBefore = sBefore.character.ac;
    const armor = sBefore.inventory.items.find((i: any) => i.item_type === 'armor');
    expect(armor).toBeTruthy();

    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: armor.name });
    expect(r.status).toBe(200);
    const s = getState(r.data);

    expect(s.character.ac).toBeGreaterThan(acBefore);
    expect(s.equipment.chest).toBeTruthy();
  });

  test('equip swaps when slot is occupied', async () => {
    // Give a new weapon to inventory
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'rapier' });

    const before = await api('GET', `/api/adventures/${advId}`);
    const oldWeapon = getState(before.data).equipment.main_hand?.name;

    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Rapier' });
    expect(r.status).toBe(200);
    const s = getState(r.data);

    // New weapon equipped
    expect(s.equipment.main_hand.name).toContain('Rapier');

    // Old weapon should be back in inventory
    if (oldWeapon) {
      const displaced = s.inventory.items.find((i: any) => i.name === oldWeapon);
      expect(displaced).toBeTruthy();
    }
  });

  test('equip non-existent item returns error', async () => {
    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Nonexistent Sword of Doom' });
    // Should still return 200 but with failure in result
    expect(r.status).toBe(200);
    const result = r.data.result || r.data;
    expect(result.success).toBe(false);
  });

  test('unequip empty slot returns error', async () => {
    // Make sure legs slot is empty
    const before = await api('GET', `/api/adventures/${advId}`);
    expect(getState(before.data).equipment.legs).toBeFalsy();

    const r = await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'legs' });
    expect(r.status).toBe(200);
    const result = r.data.result || r.data;
    expect(result.success).toBe(false);
  });

  test('equip non-equippable item returns error', async () => {
    // Give a potion (not equippable)
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'health_potion' });

    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Health Potion' });
    expect(r.status).toBe(200);
    const result = r.data.result || r.data;
    expect(result.success).toBe(false);
  });

  test('equip shield to off_hand', async () => {
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'shield' });

    const r = await api('POST', `/api/adventures/${advId}/equip`, { item_name: 'Shield' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.equipment.off_hand).toBeTruthy();
    expect(s.equipment.off_hand.name).toContain('Shield');
  });

  test('unequip off_hand returns shield to inventory', async () => {
    const r = await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'off_hand' });
    expect(r.status).toBe(200);
    const s = getState(r.data);
    expect(s.equipment.off_hand).toBeFalsy();
    const shield = s.inventory.items.find((i: any) => i.name.includes('Shield'));
    expect(shield).toBeTruthy();
  });

  test('multiple equip/unequip cycles preserve item count', async () => {
    // Count total equippable items (equipped + inventory weapons/armor)
    const before = await api('GET', `/api/adventures/${advId}`);
    const sb = getState(before.data);
    const countItems = (s: any) => {
      let count = s.inventory.items.reduce((sum: number, i: any) => sum + (i.quantity || 1), 0);
      for (const [, val] of Object.entries(s.equipment)) {
        if (val) count++;
      }
      return count;
    };
    const totalBefore = countItems(sb);

    // Unequip and re-equip main_hand
    await api('POST', `/api/adventures/${advId}/unequip`, { slot: 'main_hand' });
    const mid = await api('GET', `/api/adventures/${advId}`);
    expect(countItems(getState(mid.data))).toBe(totalBefore);

    // Re-equip
    const weapon = getState(mid.data).inventory.items.find((i: any) => i.item_type === 'weapon');
    if (weapon) {
      await api('POST', `/api/adventures/${advId}/equip`, { item_name: weapon.name });
      const after = await api('GET', `/api/adventures/${advId}`);
      expect(countItems(getState(after.data))).toBe(totalBefore);
    }
  });
});
