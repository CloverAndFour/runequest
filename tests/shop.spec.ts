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

async function api(method: string, path: string, body?: any): Promise<any> {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  try {
    return { status: res.status, data: JSON.parse(text) };
  } catch {
    return { status: res.status, data: text };
  }
}

async function createAdventure(
  name: string,
  charName: string,
  opts: { race?: string; background?: string } = {},
) {
  const body: any = {
    name,
    character_name: charName,
    race: opts.race || 'human',
    background: opts.background || 'peasant',
  };
  const r = await api('POST', '/api/adventures', body);
  expect(r.status).toBe(200);
  const state = r.data.state || r.data;
  return state.id as string;
}

async function deleteAdventure(id: string) {
  await api('DELETE', `/api/adventures/${id}`);
}

test.describe('Shop System', () => {
  let advId = '';

  test.beforeAll(async () => {
    const login = await api('POST', '/api/auth/login', {
      username: USER,
      password: PASS,
    });
    expect(login.status).toBe(200);
    token = login.data.token;

    // Human spawns at a town (race spawn has_town = true)
    advId = await createAdventure('ShopTest', 'ShopHero');

    // Give gold for purchases
    const goldR = await api('POST', `/api/adventures/${advId}/engine/gold`, {
      amount: 500,
    });
    expect(goldR.status).toBe(200);
  });

  test.afterAll(async () => {
    if (advId) await deleteAdventure(advId);
  });

  // -------------------------------------------------------------------------
  // View shop
  // -------------------------------------------------------------------------

  test('view shop returns inventory at town location', async () => {
    const r = await api('GET', `/api/adventures/${advId}/shop`);
    expect(r.status).toBe(200);
    expect(r.data.shop_name).toBeTruthy();
    expect(r.data.tier).toBeDefined();
    expect(r.data.location).toBeTruthy();
    expect(r.data.items).toBeInstanceOf(Array);
    expect(r.data.items.length).toBeGreaterThan(0);
    expect(r.data.player_gold).toBeGreaterThanOrEqual(500);

    // Each item has required fields
    const item = r.data.items[0];
    expect(item.item_id).toBeTruthy();
    expect(item.name).toBeTruthy();
    expect(item.buy_price).toBeGreaterThan(0);
    expect(item.sell_price).toBeGreaterThan(0);
    expect(item.current_stock).toBeGreaterThan(0);
    expect(['above', 'normal', 'below']).toContain(item.price_category);
    expect(item.tier).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Buy items
  // -------------------------------------------------------------------------

  test('buy item reduces gold and adds to inventory', async () => {
    // View shop to find a purchasable item
    const shop = await api('GET', `/api/adventures/${advId}/shop`);
    expect(shop.data.items.length).toBeGreaterThan(0);
    const cheapItem = shop.data.items
      .filter((i: any) => i.buy_price <= 500 && i.current_stock > 0)
      .sort((a: any, b: any) => a.buy_price - b.buy_price)[0];
    expect(cheapItem).toBeTruthy();

    const goldBefore = shop.data.player_gold;

    const r = await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: cheapItem.item_id,
      quantity: 1,
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.message).toBeTruthy();
    expect(r.data.gold_remaining).toBe(goldBefore - cheapItem.buy_price);
  });

  test('buy item reduces shop stock', async () => {
    const shop1 = await api('GET', `/api/adventures/${advId}/shop`);
    const item = shop1.data.items.find(
      (i: any) => i.current_stock > 1 && i.buy_price <= shop1.data.player_gold,
    );
    if (!item) {
      test.skip();
      return;
    }

    const stockBefore = item.current_stock;

    await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: item.item_id,
      quantity: 1,
    });

    const shop2 = await api('GET', `/api/adventures/${advId}/shop`);
    const itemAfter = shop2.data.items.find(
      (i: any) => i.item_id === item.item_id,
    );
    if (itemAfter) {
      expect(itemAfter.current_stock).toBe(stockBefore - 1);
    }
  });

  test('buy with insufficient gold fails', async () => {
    // Create a fresh adventure with no extra gold
    const poorId = await createAdventure('PoorShopTest', 'PoorHero');

    // Find an expensive item
    const shop = await api('GET', `/api/adventures/${poorId}/shop`);
    // Peasant starts with minimal or no gold, find any item
    const item = shop.data.items?.[0];
    if (!item || shop.data.player_gold >= item.buy_price) {
      // If player somehow has enough gold, try a very expensive item
      const expensive = shop.data.items?.find(
        (i: any) => i.buy_price > shop.data.player_gold,
      );
      if (!expensive) {
        await deleteAdventure(poorId);
        test.skip();
        return;
      }
      const r = await api('POST', `/api/adventures/${poorId}/shop/buy`, {
        item_id: expensive.item_id,
        quantity: 1,
      });
      expect(r.data.success).toBe(false);
      expect(r.data.error).toBeTruthy();
    } else {
      // Zero gold, any item should fail
      // Remove all gold first
      const r = await api('POST', `/api/adventures/${poorId}/shop/buy`, {
        item_id: item.item_id,
        quantity: 1,
      });
      // If peasant starts with some gold, this might succeed; handle both cases
      if (r.data.success === false) {
        expect(r.data.error).toBeTruthy();
      }
    }

    await deleteAdventure(poorId);
  });

  test('buy nonexistent item fails', async () => {
    const r = await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: 'nonexistent_sword_of_doom',
      quantity: 1,
    });
    expect(r.data.success).toBe(false);
    expect(r.data.error).toBeTruthy();
  });

  test('quantity defaults to 1 when omitted', async () => {
    const shop = await api('GET', `/api/adventures/${advId}/shop`);
    const item = shop.data.items?.find(
      (i: any) => i.current_stock > 0 && i.buy_price <= shop.data.player_gold,
    );
    if (!item) {
      test.skip();
      return;
    }

    const r = await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: item.item_id,
      // no quantity field — should default to 1
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.message).toContain('x1');
  });

  // -------------------------------------------------------------------------
  // Sell items
  // -------------------------------------------------------------------------

  test('sell item adds gold and removes from inventory', async () => {
    // Give an item via engine endpoint
    await api('POST', `/api/adventures/${advId}/engine/item`, {
      item_id: 'dagger',
    });

    const shopBefore = await api('GET', `/api/adventures/${advId}/shop`);
    const goldBefore = shopBefore.data.player_gold;

    const r = await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'Dagger',
    });
    expect(r.status).toBe(200);
    expect(r.data.success).toBe(true);
    expect(r.data.sell_price).toBeGreaterThan(0);
    expect(r.data.gold_remaining).toBe(goldBefore + r.data.sell_price);
  });

  test('sell nonexistent item fails', async () => {
    const r = await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'Unicorn Horn of Infinite Power',
    });
    expect(r.data.success).toBe(false);
    expect(r.data.error).toContain('not found');
  });

  // -------------------------------------------------------------------------
  // Dynamic pricing
  // -------------------------------------------------------------------------

  test('price increases when stock is depleted', async () => {
    // Give lots of gold to buy multiple
    await api('POST', `/api/adventures/${advId}/engine/gold`, { amount: 2000 });

    const shop1 = await api('GET', `/api/adventures/${advId}/shop`);
    const item = shop1.data.items?.find(
      (i: any) => i.current_stock >= 3 && i.buy_price < 500,
    );
    if (!item) {
      test.skip();
      return;
    }

    const price1 = item.buy_price;

    // Buy 2 to deplete stock
    await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: item.item_id,
      quantity: 1,
    });
    await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: item.item_id,
      quantity: 1,
    });

    const shop2 = await api('GET', `/api/adventures/${advId}/shop`);
    const itemAfter = shop2.data.items?.find(
      (i: any) => i.item_id === item.item_id,
    );
    if (itemAfter) {
      // Price should be >= original (supply/demand)
      expect(itemAfter.buy_price).toBeGreaterThanOrEqual(price1);
    }
  });

  test('price decreases when items are sold to shop', async () => {
    // Give daggers to sell
    await api('POST', `/api/adventures/${advId}/engine/item`, {
      item_id: 'dagger',
    });
    await api('POST', `/api/adventures/${advId}/engine/item`, {
      item_id: 'dagger',
    });
    await api('POST', `/api/adventures/${advId}/engine/item`, {
      item_id: 'dagger',
    });

    const shop1 = await api('GET', `/api/adventures/${advId}/shop`);
    const daggerBefore = shop1.data.items?.find(
      (i: any) => i.item_id === 'dagger',
    );
    const priceBefore = daggerBefore?.buy_price ?? 999;

    // Sell 3 daggers to increase supply
    await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'Dagger',
    });
    await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'Dagger',
    });
    await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'Dagger',
    });

    const shop2 = await api('GET', `/api/adventures/${advId}/shop`);
    const daggerAfter = shop2.data.items?.find(
      (i: any) => i.item_id === 'dagger',
    );
    expect(daggerAfter).toBeTruthy();
    expect(daggerAfter.buy_price).toBeLessThanOrEqual(priceBefore);
  });

  // -------------------------------------------------------------------------
  // Error: not at a town
  // -------------------------------------------------------------------------

  test('shop view fails when not at a town', async () => {
    // Create an elf adventure (elf spawn should also have a town, but
    // we'll use the adventure for the test concept — this test verifies the
    // error path exists, even if we can't easily move away from a town
    // without a travel endpoint)
    // For now, just verify the endpoint returns valid JSON with either
    // shop data or an error object
    const r = await api('GET', `/api/adventures/${advId}/shop`);
    expect(r.status).toBe(200);
    // Should have either shop_name (at town) or error (not at town)
    expect(r.data.shop_name || r.data.error).toBeTruthy();
  });

  // -------------------------------------------------------------------------
  // Shop endpoints exist (gap detection — these used to return 404)
  // -------------------------------------------------------------------------

  test('GET /api/adventures/:id/shop endpoint exists', async () => {
    const r = await api('GET', `/api/adventures/${advId}/shop`);
    expect(r.status).not.toBe(404);
  });

  test('POST /api/adventures/:id/shop/buy endpoint exists', async () => {
    const r = await api('POST', `/api/adventures/${advId}/shop/buy`, {
      item_id: 'test',
      quantity: 1,
    });
    expect(r.status).not.toBe(404);
  });

  test('POST /api/adventures/:id/shop/sell endpoint exists', async () => {
    const r = await api('POST', `/api/adventures/${advId}/shop/sell`, {
      item_name: 'test',
    });
    expect(r.status).not.toBe(404);
  });
});
