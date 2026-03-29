import { test, expect, Page } from '@playwright/test';

const TEST_USER = 'test-user';
const TEST_PASS = 'test-password1';

test.describe('Login Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Clear any stored tokens
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rq_token');
      localStorage.removeItem('rq_username');
    });
  });

  test('login page loads with correct title and form', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('h1')).toHaveText('RuneQuest');
    await expect(page.locator('#username')).toBeVisible();
    await expect(page.locator('#password')).toBeVisible();
    await expect(page.locator('.btn-login')).toBeVisible();
  });

  test('invalid credentials show error message', async ({ page }) => {
    await page.goto('/login');
    await page.fill('#username', 'nobody');
    await page.fill('#password', 'wrongpassword');
    await page.click('.btn-login');
    // Wait for error to appear (auth has 1s delay)
    await expect(page.locator('#errorMsg')).toHaveText(/Invalid credentials|realm rejects/, { timeout: 5000 });
  });

  test('successful login redirects to main page', async ({ page }) => {
    await page.goto('/login');
    await page.fill('#username', TEST_USER);
    await page.fill('#password', TEST_PASS);
    await page.click('.btn-login');

    // Should navigate to / and show the adventure select screen
    await page.waitForURL('/', { timeout: 10000 });

    // Token should be stored
    const token = await page.evaluate(() => localStorage.getItem('rq_token'));
    expect(token).toBeTruthy();
  });

  test('visiting / without token redirects to login via JS', async ({ page }) => {
    await page.goto('/');
    // The SPA should detect no token and redirect to /login
    await page.waitForURL('/login', { timeout: 5000 });
  });

  test('after login, refreshing / stays on main page', async ({ page }) => {
    // Login first
    await login(page);

    // Navigate to /
    await page.goto('/');
    // Should NOT redirect to /login
    await page.waitForTimeout(2000);
    expect(page.url()).not.toContain('/login');

    // Should show RuneQuest heading or adventure content
    const body = await page.textContent('body');
    expect(body).toContain('RuneQuest');
  });
});

async function login(page: Page) {
  await page.goto('/login');
  await page.fill('#username', TEST_USER);
  await page.fill('#password', TEST_PASS);
  await page.click('.btn-login');
  await page.waitForURL('/', { timeout: 10000 });
}
