/**
 * RuneQuest Account Management Test Suite
 * Tests change password and API key management endpoints.
 * Run: npx playwright test tests/account.spec.ts
 */
import { test, expect } from '@playwright/test';

const API = 'http://localhost:2998';
const USER = 'test-user';
const PASS = 'test-password1';

let token = '';

async function api(method: string, path: string, body?: any, authToken?: string): Promise<any> {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(authToken || token ? { 'Authorization': `Bearer ${authToken || token}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  try { return { status: res.status, data: JSON.parse(text) }; }
  catch { return { status: res.status, data: text }; }
}

async function login(password = PASS) {
  const r = await api('POST', '/api/auth/login', { username: USER, password });
  expect(r.status).toBe(200);
  token = r.data.token;
}

test.describe('Account Management', () => {
  test.beforeAll(async () => {
    await login();
  });

  test.describe('Change Password', () => {
    test('rejects wrong current password', async () => {
      const r = await api('POST', '/api/auth/change-password', {
        current_password: 'wrong-password',
        new_password: 'new-password-123',
      });
      expect(r.status).toBe(400);
      expect(r.data.error).toContain('Authentication failed');
    });

    test('rejects short new password', async () => {
      const r = await api('POST', '/api/auth/change-password', {
        current_password: PASS,
        new_password: 'short',
      });
      expect(r.status).toBe(400);
      expect(r.data.error).toContain('at least 8 characters');
    });

    test('changes password and can login with new password', async () => {
      const newPass = 'new-test-password-123';

      // Change password
      const r = await api('POST', '/api/auth/change-password', {
        current_password: PASS,
        new_password: newPass,
      });
      expect(r.status).toBe(200);
      expect(r.data.success).toBe(true);

      // Login with new password
      const loginR = await api('POST', '/api/auth/login', {
        username: USER,
        password: newPass,
      });
      expect(loginR.status).toBe(200);

      // Old password should fail
      const oldLoginR = await api('POST', '/api/auth/login', {
        username: USER,
        password: PASS,
      });
      expect(oldLoginR.status).toBe(401);

      // Change back to original password
      token = loginR.data.token;
      const revertR = await api('POST', '/api/auth/change-password', {
        current_password: newPass,
        new_password: PASS,
      });
      expect(revertR.status).toBe(200);

      // Re-login with original password
      await login();
    });
  });

  test.describe('API Keys', () => {
    test('list keys is initially manageable', async () => {
      const r = await api('GET', '/api/auth/api-keys');
      expect(r.status).toBe(200);
      expect(Array.isArray(r.data)).toBe(true);
    });

    test('create API key returns key with rq_ prefix', async () => {
      const r = await api('POST', '/api/auth/api-keys', { name: 'test-key-1' });
      expect(r.status).toBe(200);
      expect(r.data.key).toMatch(/^rq_[0-9a-f]{32}$/);
      expect(r.data.name).toBe('test-key-1');
      expect(r.data.id).toBeTruthy();
      expect(r.data.prefix).toMatch(/^rq_/);
    });

    test('API key can authenticate requests', async () => {
      // Create a key
      const createR = await api('POST', '/api/auth/api-keys', { name: 'auth-test-key' });
      expect(createR.status).toBe(200);
      const apiKey = createR.data.key;

      // Use the API key to list adventures
      const advR = await api('GET', '/api/adventures', undefined, apiKey);
      expect(advR.status).toBe(200);
      expect(Array.isArray(advR.data.adventures)).toBe(true);

      // Clean up
      await api('DELETE', `/api/auth/api-keys/${createR.data.id}`);
    });

    test('revoke API key stops authentication', async () => {
      // Create a key
      const createR = await api('POST', '/api/auth/api-keys', { name: 'revoke-test-key' });
      const apiKey = createR.data.key;
      const keyId = createR.data.id;

      // Verify it works
      const r1 = await api('GET', '/api/adventures', undefined, apiKey);
      expect(r1.status).toBe(200);

      // Revoke it
      const revokeR = await api('DELETE', `/api/auth/api-keys/${keyId}`);
      expect(revokeR.status).toBe(200);

      // Verify it no longer works
      const r2 = await api('GET', '/api/adventures', undefined, apiKey);
      expect(r2.status).toBe(401);
    });

    test('rejects empty key name', async () => {
      const r = await api('POST', '/api/auth/api-keys', { name: '' });
      expect(r.status).toBe(400);
    });

    test('list shows created keys without plaintext', async () => {
      // Create a key
      const createR = await api('POST', '/api/auth/api-keys', { name: 'list-test-key' });
      expect(createR.status).toBe(200);

      // List keys
      const listR = await api('GET', '/api/auth/api-keys');
      expect(listR.status).toBe(200);
      const found = listR.data.find((k: any) => k.name === 'list-test-key');
      expect(found).toBeTruthy();
      expect(found.prefix).toMatch(/^rq_/);
      // Should NOT contain the full key
      expect(found.key).toBeUndefined();

      // Clean up
      await api('DELETE', `/api/auth/api-keys/${createR.data.id}`);
    });

    test('revoke nonexistent key returns error', async () => {
      const r = await api('DELETE', '/api/auth/api-keys/nonexistent-id');
      expect(r.status).toBe(400);
    });

    // Clean up any remaining test keys
    test.afterAll(async () => {
      const listR = await api('GET', '/api/auth/api-keys');
      if (listR.status === 200 && Array.isArray(listR.data)) {
        for (const key of listR.data) {
          if (key.name.includes('test-key')) {
            await api('DELETE', `/api/auth/api-keys/${key.id}`);
          }
        }
      }
    });
  });

  test.describe('Unauthenticated access', () => {
    test('change password requires auth', async () => {
      const r = await api('POST', '/api/auth/change-password', {
        current_password: PASS,
        new_password: 'whatever',
      }, 'invalid-token');
      expect(r.status).toBe(401);
    });

    test('list API keys requires auth', async () => {
      const r = await api('GET', '/api/auth/api-keys', undefined, 'invalid-token');
      expect(r.status).toBe(401);
    });

    test('create API key requires auth', async () => {
      const r = await api('POST', '/api/auth/api-keys', { name: 'hack' }, 'invalid-token');
      expect(r.status).toBe(401);
    });
  });
});
