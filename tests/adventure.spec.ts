import { test, expect, Page } from '@playwright/test';

const TEST_USER = 'test-user';
const TEST_PASS = 'test-password1';

test.describe('Adventure Management', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('adventure select shows for authenticated user', async ({ page }) => {
    // After login, should see the select screen
    await expect(page.locator('.select-screen')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('h1')).toHaveText('RuneQuest');
  });

  test('new adventure button opens character creation', async ({ page }) => {
    await expect(page.locator('#newAdventureBtn')).toBeVisible({ timeout: 10000 });
    await page.click('#newAdventureBtn');

    // Should show character creation form
    await expect(page.locator('.create-screen')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('h2')).toHaveText('Create Your Adventurer');
    await expect(page.locator('#advName')).toBeVisible();
    await expect(page.locator('#charName')).toBeVisible();
    await expect(page.locator('#charRace')).toBeVisible();
    await expect(page.locator('#charClass')).toBeVisible();
  });

  test('character creation has working point buy system', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    // Default: all stats at 10, each costs 2 points (6 stats * 2 = 12), so 27 - 12 = 15 remaining
    const pointsLeft = await page.textContent('#pointsLeft');
    expect(parseInt(pointsLeft || '0')).toBe(15);

    // Increase STR to 15 (cost: 9 points instead of 2, difference = +7)
    await page.fill('#statStr', '15');
    await page.dispatchEvent('#statStr', 'input');
    const newPoints = await page.textContent('#pointsLeft');
    expect(parseInt(newPoints || '0')).toBe(8); // 15 - 7 = 8
  });

  test('creating an adventure transitions to gameplay', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await page.fill('#advName', 'Test Quest');
    await page.fill('#charName', 'TestHero');
    await page.selectOption('#charRace', 'elf');
    await page.selectOption('#charClass', 'mage');

    // Submit form
    await page.click('.create-form .stone-btn');

    // Should transition to adventure screen with two-pane layout
    await expect(page.locator('.adventure-layout')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.story-panel')).toBeVisible();
    await expect(page.locator('.info-panel')).toBeVisible();
  });
});

test.describe('Adventure Screen UI', () => {
  test.beforeEach(async ({ page }) => {
    // Login and create a quick adventure
    await login(page);
    await createAdventure(page, 'UI Test Quest', 'UIHero', 'human', 'warrior');
  });

  test('info panel tabs switch correctly', async ({ page }) => {
    await expect(page.locator('.info-panel')).toBeVisible({ timeout: 15000 });

    // Stats tab should be active by default
    await expect(page.locator('.info-tab.active')).toHaveText('Stats');

    // Click Items tab
    await page.click('.info-tab[data-tab="inventory"]');
    await expect(page.locator('.info-tab[data-tab="inventory"]')).toHaveClass(/active/);

    // Click Skills tab
    await page.click('.info-tab[data-tab="abilities"]');
    await expect(page.locator('.info-tab[data-tab="abilities"]')).toHaveClass(/active/);

    // Click Quests tab
    await page.click('.info-tab[data-tab="quests"]');
    await expect(page.locator('.info-tab[data-tab="quests"]')).toHaveClass(/active/);
  });

  test('stats tab shows character information', async ({ page }) => {
    await expect(page.locator('.info-panel')).toBeVisible({ timeout: 15000 });

    // Wait for state update to populate stats
    await expect(page.locator('.char-name')).toBeVisible({ timeout: 10000 });
    const charName = await page.textContent('.char-name');
    expect(charName).toContain('UIHero');

    // Should show stat boxes
    await expect(page.locator('.stat-box')).toHaveCount(6);
  });

  test('story panel shows narrative content', async ({ page }) => {
    // Wait for the adventure to load and LLM to start narrating
    await expect(page.locator('.story-content')).toBeVisible({ timeout: 15000 });

    // Wait for some narrative content to appear (from LLM)
    await page.waitForSelector('.narrative-block', { timeout: 30000 });
    const narrativeBlocks = await page.locator('.narrative-block').count();
    expect(narrativeBlocks).toBeGreaterThan(0);
  });
});

async function login(page: Page) {
  await page.goto('/login');
  await page.evaluate(() => {
    localStorage.removeItem('rq_token');
    localStorage.removeItem('rq_username');
  });
  await page.goto('/login');
  await page.fill('#username', TEST_USER);
  await page.fill('#password', TEST_PASS);
  await page.click('.btn-login');
  await page.waitForURL('/', { timeout: 10000 });
}

async function createAdventure(page: Page, name: string, charName: string, race: string, cls: string) {
  await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
  await page.click('#newAdventureBtn');
  await page.waitForSelector('.create-screen', { timeout: 5000 });
  await page.fill('#advName', name);
  await page.fill('#charName', charName);
  await page.selectOption('#charRace', race);
  await page.selectOption('#charClass', cls);
  await page.click('.create-form .stone-btn');
  await page.waitForSelector('.adventure-layout', { timeout: 15000 });
}
