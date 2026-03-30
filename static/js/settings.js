// Account settings panel (change password + API keys)

let wsSend = null;
let settingsModal = null;

export function initSettings(sendFn) {
    wsSend = sendFn;
}

export function showSettingsModal() {
    if (settingsModal) { settingsModal.remove(); settingsModal = null; return; }

    settingsModal = document.createElement('div');
    settingsModal.className = 'settings-overlay';
    settingsModal.innerHTML = `
        <div class="settings-panel">
            <div class="settings-header">
                <h2>Account Settings</h2>
                <button class="settings-close" id="settingsClose">&times;</button>
            </div>

            <div class="settings-section">
                <h3>Change Password</h3>
                <form id="changePasswordForm" class="settings-form">
                    <input type="password" id="currentPassword" placeholder="Current password" required>
                    <input type="password" id="newPassword" placeholder="New password (min 8 chars)" required minlength="8">
                    <input type="password" id="confirmPassword" placeholder="Confirm new password" required>
                    <div id="passwordError" class="settings-error"></div>
                    <div id="passwordSuccess" class="settings-success"></div>
                    <button type="submit" class="stone-btn">Change Password</button>
                </form>
            </div>

            <div class="settings-section">
                <h3>API Keys</h3>
                <p class="settings-hint">API keys allow programs to control your characters via the REST API. Keys are shown once at creation -- store them securely.</p>
                <div id="apiKeysList" class="api-keys-list">
                    <div class="loading">Loading keys...</div>
                </div>
                <div class="api-key-create">
                    <input type="text" id="apiKeyName" placeholder="Key name (e.g. my-bot)" maxlength="64">
                    <button class="stone-btn" id="createApiKeyBtn">Create Key</button>
                </div>
                <div id="apiKeyCreated" class="api-key-created-banner" style="display:none;"></div>
            </div>
        </div>
    `;

    document.body.appendChild(settingsModal);

    // Close handlers
    document.getElementById('settingsClose').addEventListener('click', closeSettings);
    settingsModal.addEventListener('click', (e) => { if (e.target === settingsModal) closeSettings(); });

    // Change password
    document.getElementById('changePasswordForm').addEventListener('submit', (e) => {
        e.preventDefault();
        const cur = document.getElementById('currentPassword').value;
        const newPw = document.getElementById('newPassword').value;
        const confirm = document.getElementById('confirmPassword').value;
        const errEl = document.getElementById('passwordError');
        const successEl = document.getElementById('passwordSuccess');
        errEl.textContent = '';
        successEl.textContent = '';

        if (newPw !== confirm) {
            errEl.textContent = 'Passwords do not match';
            return;
        }
        if (newPw.length < 8) {
            errEl.textContent = 'Password must be at least 8 characters';
            return;
        }
        wsSend({ type: 'change_password', current_password: cur, new_password: newPw });
    });

    // Create API key
    document.getElementById('createApiKeyBtn').addEventListener('click', () => {
        const name = document.getElementById('apiKeyName').value.trim();
        if (!name) return;
        wsSend({ type: 'create_api_key', name });
    });

    // Request API keys list
    wsSend({ type: 'list_api_keys' });
}

function closeSettings() {
    if (settingsModal) { settingsModal.remove(); settingsModal = null; }
}

export function handlePasswordChanged() {
    const successEl = document.getElementById('passwordSuccess');
    if (successEl) successEl.textContent = 'Password changed successfully!';
    const form = document.getElementById('changePasswordForm');
    if (form) form.reset();
}

export function handleApiKeyCreated(msg) {
    const banner = document.getElementById('apiKeyCreated');
    if (banner) {
        banner.style.display = 'block';
        banner.innerHTML = `
            <div class="api-key-warning">New API key created. Copy it now -- it will not be shown again!</div>
            <div class="api-key-value" id="apiKeyValue">${msg.key}</div>
            <button class="stone-btn" id="copyApiKeyBtn">Copy to Clipboard</button>
        `;
        document.getElementById('copyApiKeyBtn')?.addEventListener('click', () => {
            navigator.clipboard.writeText(msg.key).then(() => {
                document.getElementById('copyApiKeyBtn').textContent = 'Copied!';
            });
        });
    }
    // Refresh the list
    wsSend({ type: 'list_api_keys' });
    // Clear the name input
    const nameInput = document.getElementById('apiKeyName');
    if (nameInput) nameInput.value = '';
}

export function handleApiKeyList(msg) {
    const container = document.getElementById('apiKeysList');
    if (!container) return;

    if (!msg.keys || msg.keys.length === 0) {
        container.innerHTML = '<div class="empty-state">No API keys yet.</div>';
        return;
    }

    container.innerHTML = msg.keys.map(k => `
        <div class="api-key-row">
            <div class="api-key-info">
                <span class="api-key-name">${escapeHtml(k.name)}</span>
                <span class="api-key-prefix">${escapeHtml(k.prefix)}...</span>
                <span class="api-key-date">Created ${formatDate(k.created_at)}${k.last_used ? ' / Used ' + formatDate(k.last_used) : ''}</span>
            </div>
            <button class="stone-btn danger api-key-revoke" data-id="${escapeHtml(k.id)}">Revoke</button>
        </div>
    `).join('');

    container.querySelectorAll('.api-key-revoke').forEach(btn => {
        btn.addEventListener('click', () => {
            if (confirm('Revoke this API key? Programs using it will lose access.')) {
                wsSend({ type: 'revoke_api_key', key_id: btn.dataset.id });
            }
        });
    });
}

export function handleApiKeyRevoked(msg) {
    // Refresh the list
    wsSend({ type: 'list_api_keys' });
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}

function formatDate(isoStr) {
    try {
        const d = new Date(isoStr);
        return d.toLocaleDateString();
    } catch { return isoStr; }
}
