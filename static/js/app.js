// Main SPA entry point

import { getToken, getMe, getWsUrl, clearToken } from './api.js';
import { WebSocketManager } from './ws.js';
import { renderSelectScreen, renderCreateScreen } from './select.js';
import { renderAdventure } from './adventure.js';
import { renderCombatStarted, renderCombatTurnStart, renderCombatActionResult, renderEnemyTurn, renderCombatEnded } from './combat.js';

const app = document.getElementById('app');
let ws = null;
let currentView = null;
let gameState = null;
let currentModel = 'grok-4-1-fast-reasoning';

async function init() {
    const token = getToken();
    if (!token) {
        window.location.href = '/login';
        return;
    }

    try {
        await getMe();
    } catch {
        window.location.href = '/login';
        return;
    }

    connectWebSocket();
}

function connectWebSocket() {
    ws = new WebSocketManager(getWsUrl(), {
        onOpen: () => {
            showSelectScreen();
        },
        onMessage: (msg) => handleServerMsg(msg),
        onClose: () => {},
        onError: () => {},
    });
    ws.connect();
}

function handleServerMsg(msg) {
    switch (msg.type) {
        case 'connected':
            break;
        case 'adventure_list':
            if (currentView === 'select') {
                renderSelectScreen(app, msg.adventures, {
                    onLoad: (id) => ws.send({ type: 'load_adventure', adventure_id: id }),
                    onDelete: (id) => ws.send({ type: 'delete_adventure', adventure_id: id }),
                    onNew: () => showCreateScreen(),
                });
            }
            break;
        case 'adventure_created':
        case 'adventure_loaded':
            if (msg.state) gameState = msg.state;
            showAdventureScreen();
            break;
        case 'chat_history':
            renderChatHistory(msg.entries);
            break;
        case 'narrative_chunk':
            appendNarrativeChunk(msg.text);
            break;
        case 'narrative_end':
            endNarrative();
            break;
        case 'dice_roll_request':
            showDiceRollUI(msg);
            break;
        case 'dice_roll_result':
            showDiceResult(msg);
            break;
        case 'present_choices':
            showChoices(msg);
            break;
        case 'state_update':
            gameState = msg.state;
            updateInfoPanel();
            break;
        case 'cost_update':
            updateCostDisplay(msg);
            break;
        case 'condition_effects':
            showConditionEffects(msg.effects);
            break;
        case 'model_info':
            currentModel = msg.model;
            break;
        case 'combat_started':
            renderCombatStarted(app, msg);
            break;
        case 'combat_turn_start':
            renderCombatTurnStart(app, msg, (actionId, target) => {
                ws.send({ type: 'combat_action', action_id: actionId, target: target || null });
            });
            break;
        case 'combat_action_result':
            renderCombatActionResult(app, msg);
            break;
        case 'combat_enemy_turn':
            renderEnemyTurn(app, msg);
            break;
        case 'combat_ended':
            renderCombatEnded(app, msg);
            break;
        case 'error':
            showToast(msg.message, true);
            break;
    }
}

function showSelectScreen() {
    currentView = 'select';
    ws.send({ type: 'list_adventures' });
    app.innerHTML = '<div class="select-screen"><h1>RuneQuest</h1><div class="loading">Loading adventures</div></div>';
}

function showCreateScreen() {
    currentView = 'create';
    renderCreateScreen(app, {
        onBack: () => showSelectScreen(),
        onCreate: (data) => ws.send({
            type: 'create_adventure',
            ...data,
        }),
    });
}

function showAdventureScreen() {
    currentView = 'adventure';
    renderAdventure(app, gameState, {
        onSendMessage: (text) => ws.send({ type: 'send_message', content: text }),
        onSelectChoice: (index, text) => ws.send({ type: 'select_choice', index, text }),
        onRollDice: () => ws.send({ type: 'roll_dice' }),
        onBack: () => showSelectScreen(),
        onGetStats: () => ws.send({ type: 'get_character_sheet' }),
    });

    // Listen for options button
    document.addEventListener('show-options', showOptionsModal);
}

// Narrative streaming
let currentNarrativeEl = null;

function appendNarrativeChunk(text) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    // Remove loading placeholder if present
    const loadingEl = storyContent.querySelector('.loading-narrative');
    if (loadingEl) loadingEl.remove();

    if (!currentNarrativeEl) {
        currentNarrativeEl = document.createElement('div');
        currentNarrativeEl.className = 'narrative-block streaming';
        storyContent.appendChild(currentNarrativeEl);
    }

    currentNarrativeEl.textContent += text;
    storyContent.scrollTop = storyContent.scrollHeight;
}

function endNarrative() {
    if (currentNarrativeEl) {
        currentNarrativeEl.classList.remove('streaming');
        currentNarrativeEl = null;
    }
}

function showDiceRollUI(data) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    endNarrative();

    const div = document.createElement('div');
    div.className = 'dice-roll-ui';
    div.innerHTML = `
        <div class="dice-description">${escapeHtml(data.description)}</div>
        <div class="dice-info">
            <div>Dice: <span>${data.count}${data.dice_type}</span></div>
            <div>Modifier: <span>${data.modifier >= 0 ? '+' : ''}${data.modifier}</span></div>
            <div>DC: <span>${data.dc}</span></div>
        </div>
        <div class="probability">Success chance: ${Math.round(data.success_probability * 100)}%</div>
        <button class="btn-roll-dice" onclick="document.dispatchEvent(new Event('roll-dice'))">Roll Dice</button>
    `;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;

    const handler = () => {
        ws.send({ type: 'roll_dice' });
        const btn = div.querySelector('.btn-roll-dice');
        btn.disabled = true;
        // Replace button with rolling animation
        div.innerHTML = `
            <div class="dice-description">${escapeHtml(data.description)}</div>
            <div class="dice-rolling-animation">
                <div class="rolling-dice">
                    <span class="dice-face"></span>
                </div>
                <div class="rolling-text">Rolling...</div>
            </div>
        `;
        // Animate random numbers on the dice face
        const face = div.querySelector('.dice-face');
        const animInterval = setInterval(() => {
            face.textContent = Math.floor(Math.random() * 20) + 1;
        }, 80);
        // Store interval so we can clear it when result arrives
        div.dataset.animInterval = animInterval;
        document.removeEventListener('roll-dice', handler);
    };
    document.addEventListener('roll-dice', handler);
}

function showDiceResult(data) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    // Clear rolling animation
    const rollUI = storyContent.querySelector('.dice-roll-ui');
    if (rollUI) {
        if (rollUI.dataset.animInterval) clearInterval(parseInt(rollUI.dataset.animInterval));
        rollUI.remove();
    }

    const div = document.createElement('div');
    div.className = `dice-result ${data.success ? 'success' : 'failure'}`;
    div.innerHTML = `
        <div>Rolled: ${data.rolls.join(' + ')} = <strong>${data.total}</strong> vs DC ${data.dc}</div>
        <div>${data.success ? 'SUCCESS!' : 'FAILURE'}</div>
    `;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

function showChoices(data) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    endNarrative();

    const div = document.createElement('div');
    div.className = 'choices-container';

    let html = `<div class="choices-prompt">${escapeHtml(data.prompt)}</div><div class="choices-grid">`;
    data.choices.forEach((choice, i) => {
        html += `<button class="choice-btn" data-index="${i}" data-text="${escapeAttr(choice)}">
            <span class="choice-number">${i + 1}.</span> ${escapeHtml(choice)}
        </button>`;
    });
    html += '</div>';

    if (data.allow_custom_input) {
        html += `<div class="custom-input-area">
            <input type="text" placeholder="Or type your own action..." id="customAction">
            <button class="stone-btn" id="customActionBtn">Go</button>
        </div>`;
    }

    div.innerHTML = html;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;

    div.querySelectorAll('.choice-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const index = parseInt(btn.dataset.index);
            const text = btn.dataset.text;
            div.remove();
            ws.send({ type: 'select_choice', index, text });
        });
    });

    if (data.allow_custom_input) {
        const input = div.querySelector('#customAction');
        const btn = div.querySelector('#customActionBtn');
        const submit = () => {
            const text = input.value.trim();
            if (text) {
                div.remove();
                ws.send({ type: 'send_message', content: text });
            }
        };
        btn.addEventListener('click', submit);
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') submit();
        });
    }
}

function updateInfoPanel() {
    if (currentView !== 'adventure' || !gameState) return;
    const event = new CustomEvent('state-update', { detail: gameState });
    document.dispatchEvent(event);
}

function updateCostDisplay(data) {
    const el = document.getElementById('costDisplay');
    if (!el) return;
    const fmt = (n) => {
        if (n < 0.001) return `$${(n * 1000).toFixed(2)}m`;
        if (n < 0.01) return `$${n.toFixed(4)}`;
        if (n < 1) return `$${n.toFixed(3)}`;
        return `$${n.toFixed(2)}`;
    };
    const cost = data.session_cost_usd || 0;
    el.textContent = fmt(cost);
    el.title = `Session: ${fmt(cost)}\nToday: ${fmt(data.today_cost_usd || 0)}\nThis week: ${fmt(data.week_cost_usd || 0)}\nThis month: ${fmt(data.month_cost_usd || 0)}\nAll time: ${fmt(data.total_cost_usd || 0)}\n${data.prompt_tokens || 0} input + ${data.completion_tokens || 0} output tokens`;
}

function renderChatHistory(entries) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent || !entries || entries.length === 0) return;

    const loadingEl = storyContent.querySelector('.loading-narrative');
    if (loadingEl) loadingEl.remove();

    // Render each display event by type
    entries.forEach(entry => {
        const div = document.createElement('div');
        switch (entry.event_type) {
            case 'narrative':
                div.className = 'narrative-block history';
                div.textContent = entry.data.text || entry.data.content || '';
                if (div.textContent) storyContent.appendChild(div);
                break;
            case 'dice_result':
                div.className = `dice-result history ${entry.data.success ? 'success' : 'failure'}`;
                div.innerHTML = `Rolled: ${entry.data.total} vs DC ${entry.data.dc} — ${entry.data.success ? 'SUCCESS' : 'FAILURE'}`;
                storyContent.appendChild(div);
                break;
            case 'choices':
                // Show the choices that were presented (but don't make them clickable — they're history)
                div.className = 'choices-container history';
                div.innerHTML = `<div class="choices-prompt" style="opacity:0.6">${escapeHtml(entry.data.prompt || 'What do you do?')}</div>`;
                storyContent.appendChild(div);
                break;
            case 'choice_selected':
                div.className = 'narrative-block history';
                div.style.color = 'var(--text-gold)';
                div.textContent = `> ${entry.data.text || ''}`;
                if (div.textContent.length > 2) storyContent.appendChild(div);
                break;
            case 'combat_action':
                div.className = `combat-action-log history ${entry.data.hit === true ? 'hit' : entry.data.hit === false ? 'miss' : 'neutral'}`;
                div.textContent = entry.data.description || '';
                if (div.textContent) storyContent.appendChild(div);
                break;
            case 'combat_enemy':
                div.className = `combat-action-log history enemy ${entry.data.hit ? 'hit' : 'miss'}`;
                div.textContent = entry.data.description || `${entry.data.enemy_name} attacks — ${entry.data.hit ? 'HIT' : 'MISS'}`;
                storyContent.appendChild(div);
                break;
            default:
                // For unknown types or plain text, show as narrative
                if (entry.data && (entry.data.text || entry.data.content)) {
                    div.className = 'narrative-block history';
                    div.textContent = entry.data.text || entry.data.content;
                    storyContent.appendChild(div);
                }
                break;
        }
    });

    const sep = document.createElement('div');
    sep.className = 'history-separator';
    sep.textContent = '— continuing —';
    storyContent.appendChild(sep);

    storyContent.scrollTop = storyContent.scrollHeight;
}

function showConditionEffects(effects) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent || !effects || effects.length === 0) return;

    const div = document.createElement('div');
    div.className = 'condition-effects';
    div.innerHTML = effects.map(e => `<div class="condition-effect-line">${escapeHtml(e)}</div>`).join('');
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

function showOptionsModal() {
    const existing = document.querySelector('.options-modal');
    if (existing) { existing.remove(); return; }

    const modal = document.createElement('div');
    modal.className = 'options-modal';
    modal.innerHTML = `
        <div class="options-content">
            <h3>Options</h3>
            <div class="option-group">
                <label>Model</label>
                <select id="modelSelect">
                    <option value="grok-4-1-fast-reasoning" ${currentModel.includes('non') ? '' : 'selected'}>Grok 4.1 Reasoning (smarter)</option>
                    <option value="grok-4-1-fast-non-reasoning" ${currentModel.includes('non') ? 'selected' : ''}>Grok 4.1 Fast (quicker)</option>
                </select>
            </div>
            <button class="stone-btn" id="closeOptions" style="width:100%;margin-top:12px;">Close</button>
        </div>
    `;
    document.body.appendChild(modal);

    document.getElementById('modelSelect').addEventListener('change', (e) => {
        ws.send({ type: 'set_model', model: e.target.value });
        currentModel = e.target.value;
    });
    document.getElementById('closeOptions').addEventListener('click', () => modal.remove());
    modal.addEventListener('click', (e) => { if (e.target === modal) modal.remove(); });
}

function showToast(message, isError = false) {
    const toast = document.createElement('div');
    toast.className = `toast ${isError ? 'error' : ''}`;
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 4000);
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function escapeAttr(str) {
    return str.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

window.rqWs = { send: (msg) => ws?.send(msg) };

init();
