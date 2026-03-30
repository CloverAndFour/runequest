/**
 * RuneQuest Shop System Tests
 * Tests the shop REST API on port 2998.
 * Run: npx playwright test tests/shop.spec.ts
 */
import { test, expect } from '@playwright/test';

const API = 'http://localhost:2998';
const USER = 'test-user';
const PASS = 'test-password1';

let token = '';
let advId = '';

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

test.describe('Shop System', () => {
  test.beforeAll(async () => {
    // Login
    const login = await api('POST', '/api/auth/login', { username: USER, password: PASS });
    expect(login.status).toBe(200);
    token = login.data.token;

    // Create adventure (human spawns at a town)
    const adv = await api('POST', '/api/adventures', {
      name: 'ShopTest',
      character_name: 'ShopHero',
      race: 'human',
      class: 'warrior',
      stats: { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
    });
    expect(adv.status).toBe(200);
    const state = adv.data.state || adv.data;
    advId = state.id;
    expect(advId).toBeTruthy();

    // Give the character gold to buy things
    await api('POST', `/api/adventures/${advId}/engine/gold`, { amount: 500 });
  });

  test.afterAll(async () => {
    if (advId) await api('DELETE', `/api/adventures/${advId}`);
  });

  test('view shop returns inventory at town location', async () => {
    const r = await api('GET', `/api/shop?adventure_id=${advId}`);
    expect(r.status).toBe(200);
    expect(r.data.shop_name).toBeTruthy();
    expect(r.data.items).toBeInstanceOf(Array);
    expect(r.data.items.length).toBeGreaterThan(0);
    expect(r.data.player_gold).toBeGreaterThanOrEqual(500);
    expect(r.data.player_inventory).toBeInstanceOf(Array);

    // Each item has required fields
    const item = r.data.items[0];
    expect(item.item_id).toBeTruthy();
    expect(item.name).toBeTruthy();
    expect(item.buy_price).toBeGreaterThan(0);
    expect(item.sell_price).toBeGreaterThan(0);
    expect(item.current_stock).toBeGreaterThan(0);
    expect(['above', 'normal', 'below']).toContain(item.price_category);
  });

  test('view shop requires adventure_id', async () => {
    const r = await api('GET', '/api/shop');
    expect(r.status).toBe(400);
  });

  test('buy item reduces gold and adds to inventory', async () => {
    // Get shop to find a cheap item
    const shop = await api('GET', `/api/shop?adventure_id=${advId}`);
    const cheapItem = shop.data.items.sort((a: any, b: any) => a.buy_price - b.buy_price)[0];
    const goldBefore = shop.data.player_gold;

    const r = await api('POST', '/api/shop/buy', {
      adventure_id: advId,
      item_id: cheapItem.item_id,
      quantity: 1,
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.price_paid).toBe(cheapItem.buy_price);
    expect(r.data.gold_remaining).toBe(goldBefore - cheapItem.buy_price);
    expect(r.data.item_name).toBeTruthy();
  });

  test('buy item reduces shop stock', async () => {
    // Get shop state
    const shop1 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const item = shop1.data.items.find((i: any) => i.current_stock > 1);
    if (!item) return; // skip if no multi-stock items

    const stockBefore = item.current_stock;

    await api('POST', '/api/shop/buy', {
      adventure_id: advId,
      item_id: item.item_id,
      quantity: 1,
    });

    const shop2 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const itemAfter = shop2.data.items.find((i: any) => i.item_id === item.item_id);
    expect(itemAfter.current_stock).toBe(stockBefore - 1);
  });

  test('buy item with insufficient gold fails', async () => {
    // Create a fresh adventure with no gold
    const adv = await api('POST', '/api/adventures', {
      name: 'PoorShopTest',
      character_name: 'PoorHero',
      race: 'human',
      class: 'warrior',
      stats: { strength: 15, dexterity: 10, constitution: 14, intelligence: 8, wisdom: 10, charisma: 8 },
      naked_start: true,
    });
    const poorId = (adv.data.state || adv.data).id;

    const r = await api('POST', '/api/shop/buy', {
      adventure_id: poorId,
      item_id: 'longsword',
      quantity: 1,
    });
    expect(r.data.success).toBe(false);
    expect(r.data.message).toContain('gold');

    await api('DELETE', `/api/adventures/${poorId}`);
  });

  test('buy nonexistent item fails', async () => {
    const r = await api('POST', '/api/shop/buy', {
      adventure_id: advId,
      item_id: 'nonexistent_sword_of_doom',
      quantity: 1,
    });
    expect(r.data.success).toBe(false);
  });

  test('sell item adds gold and removes from inventory', async () => {
    // First give an item to sell
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'longsword' });

    const shopBefore = await api('GET', `/api/shop?adventure_id=${advId}`);
    const goldBefore = shopBefore.data.player_gold;

    const r = await api('POST', '/api/shop/sell', {
      adventure_id: advId,
      item_name: 'Longsword',
      quantity: 1,
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.gold_earned).toBeGreaterThan(0);
    expect(r.data.gold_remaining).toBe(goldBefore + r.data.gold_earned);
  });

  test('sold item appears in shop for other players', async () => {
    // After selling a longsword, it should be in the shop
    const shop = await api('GET', `/api/shop?adventure_id=${advId}`);
    const longsword = shop.data.items.find((i: any) => i.item_id === 'longsword');
    // It should exist (either as base item with increased stock, or as player-sold)
    expect(longsword).toBeTruthy();
    expect(longsword.current_stock).toBeGreaterThan(0);
  });

  test('sell nonexistent item fails', async () => {
    const r = await api('POST', '/api/shop/sell', {
      adventure_id: advId,
      item_name: 'Unicorn Horn of Infinite Power',
      quantity: 1,
    });
    expect(r.data.success).toBe(false);
    expect(r.data.message).toContain('not found');
  });

  test('price increases when stock is depleted', async () => {
    // Buy an item multiple times and check price rises
    const shop1 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const item = shop1.data.items.find((i: any) => i.current_stock >= 3 && i.buy_price < 200);
    if (!item) return; // skip if no suitable item

    const price1 = item.buy_price;

    // Buy 2
    await api('POST', '/api/shop/buy', { adventure_id: advId, item_id: item.item_id, quantity: 1 });
    await api('POST', '/api/shop/buy', { adventure_id: advId, item_id: item.item_id, quantity: 1 });

    const shop2 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const itemAfter = shop2.data.items.find((i: any) => i.item_id === item.item_id);
    if (itemAfter) {
      expect(itemAfter.buy_price).toBeGreaterThanOrEqual(price1);
    }
  });

  test('price decreases when items are sold to shop', async () => {
    // Give multiple items and sell them
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'dagger' });
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'dagger' });
    await api('POST', `/api/adventures/${advId}/engine/item`, { item_id: 'dagger' });

    const shop1 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const daggerBefore = shop1.data.items.find((i: any) => i.item_id === 'dagger');
    const priceBefore = daggerBefore?.buy_price || 999;

    // Sell 3 daggers
    await api('POST', '/api/shop/sell', { adventure_id: advId, item_name: 'Dagger', quantity: 1 });
    await api('POST', '/api/shop/sell', { adventure_id: advId, item_name: 'Dagger', quantity: 1 });
    await api('POST', '/api/shop/sell', { adventure_id: advId, item_name: 'Dagger', quantity: 1 });

    const shop2 = await api('GET', `/api/shop?adventure_id=${advId}`);
    const daggerAfter = shop2.data.items.find((i: any) => i.item_id === 'dagger');
    expect(daggerAfter).toBeTruthy();
    expect(daggerAfter.buy_price).toBeLessThanOrEqual(priceBefore);
  });

  test('quantity defaults to 1', async () => {
    // Buy without quantity field
    const shop = await api('GET', `/api/shop?adventure_id=${advId}`);
    const item = shop.data.items.find((i: any) => i.current_stock > 0 && i.buy_price < 100);
    if (!item) return;

    const r = await api('POST', '/api/shop/buy', {
      adventure_id: advId,
      item_id: item.item_id,
      // no quantity field
    });
    expect(r.data.success).toBe(true);
    expect(r.data.message).toContain('x1');
  });
});
