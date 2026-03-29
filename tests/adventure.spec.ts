import { test, expect, Page } from '@playwright/test';

const TEST_USER = 'test-user';
const TEST_PASS = 'test-password1';

test.describe('Adventure Management', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('adventure select shows for authenticated user', async ({ page }) => {
    await expect(page.locator('.select-screen')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('h1')).toHaveText('RuneQuest');
  });

  test('new adventure button opens character creation', async ({ page }) => {
    await expect(page.locator('#newAdventureBtn')).toBeVisible({ timeout: 10000 });
    await page.click('#newAdventureBtn');

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

    const pointsLeft = await page.textContent('#pointsLeft');
    expect(parseInt(pointsLeft || '0')).toBe(15);

    await page.fill('#statStr', '15');
    await page.dispatchEvent('#statStr', 'input');
    const newPoints = await page.textContent('#pointsLeft');
    expect(parseInt(newPoints || '0')).toBe(8);
  });

  test('creating an adventure transitions to gameplay', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await page.fill('#advName', 'Test Quest');
    await page.fill('#charName', 'TestHero');
    await page.selectOption('#charRace', 'elf');
    await page.selectOption('#charClass', 'mage');
    await page.click('.create-form .stone-btn');

    await expect(page.locator('.adventure-layout')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.story-panel')).toBeVisible();
    await expect(page.locator('.info-panel')).toBeVisible();
  });
});

test.describe('Scenario Selection', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('character creation shows scenario chooser', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await expect(page.locator('#scenarioCards')).toBeVisible();
    await expect(page.locator('.scenario-card')).toHaveCount(6);
    await expect(page.locator('#customScenario')).toBeVisible();
  });

  test('clicking a scenario card selects it', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.scenario-card', { timeout: 5000 });

    await page.click('.scenario-card[data-scenario="dragons-lair"]');
    await expect(page.locator('.scenario-card[data-scenario="dragons-lair"]')).toHaveClass(/selected/);
    await expect(page.locator('.scenario-card[data-scenario=""]')).not.toHaveClass(/selected/);
  });
});

test.describe('Stat Cost Display', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('stat inputs show cost indicators', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await expect(page.locator('#costStr')).toBeVisible();
    const costText = await page.textContent('#costStr');
    expect(costText).toContain('Cost: 2');
  });

  test('changing stat updates cost indicator', async ({ page }) => {
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await page.fill('#statStr', '15');
    await page.dispatchEvent('#statStr', 'input');
    const costText = await page.textContent('#costStr');
    expect(costText).toContain('Cost: 9');
  });
});

test.describe('Adventure Screen UI', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await createAdventure(page, 'UI Test Quest', 'UIHero', 'human', 'warrior');
  });

  test('Status tab shows stats and abilities (merged)', async ({ page }) => {
    await expect(page.locator('.info-panel')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.info-tab.active')).toHaveText('Status');
    await expect(page.locator('.stat-box')).toHaveCount(6);
    // Abilities section should be visible in the Status tab
    await expect(page.locator('.abilities-section')).toBeVisible();
  });

  test('only 3 tabs exist (Status, Items, Quests)', async ({ page }) => {
    await expect(page.locator('.info-panel')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('.info-tab')).toHaveCount(3);
  });

  test('info panel tabs switch correctly', async ({ page }) => {
    await expect(page.locator('.info-panel')).toBeVisible({ timeout: 15000 });

    await page.click('.info-tab[data-tab="inventory"]');
    await expect(page.locator('.info-tab[data-tab="inventory"]')).toHaveClass(/active/);

    await page.click('.info-tab[data-tab="quests"]');
    await expect(page.locator('.info-tab[data-tab="quests"]')).toHaveClass(/active/);
  });

  test('story panel shows narrative content', async ({ page }) => {
    await expect(page.locator('.story-content')).toBeVisible({ timeout: 15000 });
    await page.waitForSelector('.narrative-block', { timeout: 30000 });
    const narrativeBlocks = await page.locator('.narrative-block').count();
    expect(narrativeBlocks).toBeGreaterThan(0);
  });
});

test.describe('Cost Indicator', () => {
  test('cost display appears in story header', async ({ page }) => {
    await login(page);
    await createAdventure(page, 'Cost Test', 'CostHero', 'human', 'warrior');
    await expect(page.locator('#costDisplay')).toBeVisible({ timeout: 15000 });
  });
});

test.describe('Model Switching', () => {
  test('options button opens model selector', async ({ page }) => {
    await login(page);
    await createAdventure(page, 'Model Test', 'ModelHero', 'dwarf', 'cleric');
    await expect(page.locator('#optionsBtn')).toBeVisible({ timeout: 15000 });
    await page.click('#optionsBtn');
    await expect(page.locator('.options-modal')).toBeVisible();
    await expect(page.locator('#modelSelect')).toBeVisible();
  });

  test('closing options modal works', async ({ page }) => {
    await login(page);
    await createAdventure(page, 'Model Test 2', 'ModelHero2', 'dwarf', 'cleric');
    await page.click('#optionsBtn');
    await expect(page.locator('.options-modal')).toBeVisible();
    await page.click('#closeOptions');
    await expect(page.locator('.options-modal')).not.toBeVisible();
  });
});

test.describe('Loading Animation', () => {
  test('adventure shows loading animation initially', async ({ page }) => {
    await login(page);
    await page.waitForSelector('#newAdventureBtn', { timeout: 10000 });
    await page.click('#newAdventureBtn');
    await page.waitForSelector('.create-screen', { timeout: 5000 });

    await page.fill('#advName', 'Loading Test');
    await page.fill('#charName', 'LoadHero');
    await page.click('.create-form .stone-btn');

    await expect(page.locator('.loading-narrative')).toBeVisible({ timeout: 15000 });
  });

  test('loading animation disappears after narrative starts', async ({ page }) => {
    await login(page);
    await createAdventure(page, 'Loading Test 2', 'LoadHero2', 'human', 'warrior');
    await page.waitForSelector('.narrative-block:not(.loading-narrative)', { timeout: 30000 });
    await expect(page.locator('.loading-narrative')).toHaveCount(0);
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
