// Main SPA entry point

import { getToken, getMe, getWsUrl, clearToken } from './api.js';
import { WebSocketManager } from './ws.js';
import { renderSelectScreen, renderCreateScreen } from './select.js';
import { renderAdventure } from './adventure.js';
import { renderCombatStarted, renderCombatTurnStart, renderCombatActionResult, renderEnemyTurn, renderCombatEnded } from './combat.js';
import { initFriendsPanel, handleFriendCode, handleFriendsList, handleFriendPresence, handleFriendRequestReceived, handleFriendRequestAccepted, handleFriendRequestSent, handleFriendChat, handleFriendChatSent, handleFriendChatHistory } from './friends.js';
import { initLocationChat, handleLocationChat, handleLocationPresence, handleLocationChatHistory, onLocationChange } from './location-chat.js';

const app = document.getElementById('app');
let ws = null;
let currentView = null;
let gameState = null;
let currentModel = 'grok-4-1-fast-reasoning';
let loadingOverlayTimeout = null;


const LOADING_FLAVOR_TEXTS = [
    "Entering the realm...",
    "Forging your destiny...",
    "The runes align...",
    "Consulting the oracle...",
    "Opening ancient scrolls...",
    "Crossing the threshold...",
];

function showAdventureLoadingOverlay(isCreating) {
    hideAdventureLoadingOverlay();
    const flavor = LOADING_FLAVOR_TEXTS[Math.floor(Math.random() * LOADING_FLAVOR_TEXTS.length)];
    const title = isCreating ? "Creating Adventure" : "Loading Adventure";
    const overlay = document.createElement("div");
    overlay.className = "adventure-loading-overlay";
    overlay.innerHTML = `
        <div class="loading-rune"></div>
        <div class="loading-title">${title}</div>
        <div class="loading-flavor">${flavor}</div>
        <div class="loading-error">
            <p>This is taking longer than expected...</p>
            <button class="stone-btn" id="loadingRetryBtn">Back to Menu</button>
        </div>
    `;
    document.body.appendChild(overlay);

    overlay.querySelector("#loadingRetryBtn")?.addEventListener("click", () => {
        hideAdventureLoadingOverlay();
        showSelectScreen();
    });

    loadingOverlayTimeout = setTimeout(() => {
        const errorEl = overlay.querySelector(".loading-error");
        if (errorEl) errorEl.classList.add("visible");
    }, 30000);
}

function hideAdventureLoadingOverlay() {
    if (loadingOverlayTimeout) {
        clearTimeout(loadingOverlayTimeout);
        loadingOverlayTimeout = null;
    }
    const overlay = document.querySelector(".adventure-loading-overlay");
    if (overlay) overlay.remove();
}

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
                    onLoad: (id) => { showAdventureLoadingOverlay(false); ws.send({ type: 'load_adventure', adventure_id: id }); },
                    onDelete: (id) => ws.send({ type: 'delete_adventure', adventure_id: id }),
                    onNew: () => showCreateScreen(),
                });
            }
            break;
        case 'adventure_created':
        case 'adventure_loaded':
            hideAdventureLoadingOverlay();
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
            renderFixedActions();
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
            renderFixedActions();
            break;
        case 'state_changes':
            showStateChanges(msg);
            if (gameState && gameState.character) {
                if (msg.gold_delta) gameState.character.gold += msg.gold_delta;
                if (msg.xp_delta) gameState.character.xp += msg.xp_delta;
                if (msg.hp_delta) gameState.character.hp = Math.min(
                    gameState.character.hp + msg.hp_delta,
                    gameState.character.max_hp
                );
            }
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
        case 'shop_inventory':
            showShopPanel(msg);
            break;
        case 'shop_buy_result':
            handleShopBuyResult(msg);
            break;
        case 'shop_sell_result':
            handleShopSellResult(msg);
            break;
        case 'gather_result':
            showGatherResult(msg);
            break;
        case 'skill_list':
            document.dispatchEvent(new CustomEvent('skill-list-received', { detail: msg }));
            break;
        case 'friend_code':
            handleFriendCode(msg);
            break;
        case 'friends_list':
            handleFriendsList(msg);
            break;
        case 'friend_presence':
            handleFriendPresence(msg);
            break;
        case 'friend_request_received':
            handleFriendRequestReceived(msg);
            break;
        case 'friend_request_accepted':
            handleFriendRequestAccepted(msg);
            break;
        case 'friend_request_sent':
            handleFriendRequestSent(msg);
            break;
        case 'friend_chat':
            handleFriendChat(msg);
            break;
        case 'friend_chat_sent':
            handleFriendChatSent(msg);
            break;
        case 'friend_chat_history':
            handleFriendChatHistory(msg);
            break;
        case 'error':
            hideAdventureLoadingOverlay();
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
        onCreate: (data) => {
            showAdventureLoadingOverlay(true);
            ws.send({ type: 'create_adventure', ...data });
        },
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

    // Initialize friends panel
    setTimeout(() => {
        const layout = app.querySelector('.adventure-layout');
        if (layout) {
            initFriendsPanel(layout, (msg) => ws?.send(msg));
        }
    }, 100);

    // Initialize location chat
    setTimeout(() => {
        initLocationChat(app, (msg) => ws?.send(msg));
    }, 150);
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
        currentNarrativeEl.classList.remove("streaming");
        // Post-process: italicize dialogue
        currentNarrativeEl.innerHTML = formatNarrative(currentNarrativeEl.textContent);
        const storyContent = document.querySelector('.story-content');
        if (storyContent) storyContent.scrollTop = storyContent.scrollHeight;
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
        diceRollStartTime = Date.now();
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

// Queue dice results to show after minimum animation time
let pendingDiceResult = null;
let diceRollStartTime = 0;
const DICE_ANIM_MIN_MS = 2000;

function showDiceResult(data) {
    const elapsed = Date.now() - diceRollStartTime;
    const remaining = Math.max(0, DICE_ANIM_MIN_MS - elapsed);

    if (remaining > 0) {
        // Wait for animation to finish, then show
        pendingDiceResult = data;
        setTimeout(() => displayDiceResult(pendingDiceResult), remaining);
        return;
    }
    displayDiceResult(data);
}

function displayDiceResult(data) {
    pendingDiceResult = null;
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    // Clear rolling animation — show final number briefly
    const rollUI = storyContent.querySelector('.dice-roll-ui');
    if (rollUI) {
        const face = rollUI.querySelector('.dice-face');
        if (face) face.textContent = data.rolls[0] || data.total;
        if (rollUI.dataset.animInterval) clearInterval(parseInt(rollUI.dataset.animInterval));
        // Brief pause showing the final number
        setTimeout(() => rollUI.remove(), 300);
    }

    setTimeout(() => {
        const div = document.createElement('div');
        div.className = `dice-result ${data.success ? 'success' : 'failure'}`;
        div.innerHTML = `
            <div>Rolled: ${data.rolls.length > 1 ? data.rolls.join(' + ') + ' = ' : ''}<strong>${data.total}</strong> vs DC ${data.dc}</div>
            <div>${data.success ? 'SUCCESS!' : 'FAILURE'}</div>
        `;
        storyContent.appendChild(div);
        storyContent.scrollTop = storyContent.scrollHeight;
    }, 400);
}

function showChoices(data) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    endNarrative();

    const div = document.createElement('div');
    div.className = 'llm-choices-section';

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

    // Ensure fixed actions appear above LLM choices
    renderFixedActions();

    div.querySelectorAll('.choice-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const index = parseInt(btn.dataset.index);
            const text = btn.dataset.text;
            storyContent.querySelectorAll('.fixed-choices-section, .llm-choices-section, .choices-separator').forEach(function(el) { el.remove(); });
            showLoadingSpinner();
            ws.send({ type: 'select_choice', index, text });
        });
    });

    if (data.allow_custom_input) {
        const input = div.querySelector('#customAction');
        const btn = div.querySelector('#customActionBtn');
        const submit = () => {
            const text = input.value.trim();
            if (text) {
                storyContent.querySelectorAll('.fixed-choices-section, .llm-choices-section, .choices-separator').forEach(function(el) { el.remove(); });
                const storyContent2 = document.querySelector('.story-content');
                if (storyContent2) {
                    const msgBlock = document.createElement('div');
                    msgBlock.className = 'user-message-block';
                    msgBlock.textContent = '> ' + text;
                    storyContent2.appendChild(msgBlock);
                }
                showLoadingSpinner();
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
                div.innerHTML = formatNarrative(entry.data.text || entry.data.content || '');
                if (div.innerHTML) storyContent.appendChild(div);
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

function showLoadingSpinner() {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    if (storyContent.querySelector('.loading-narrative')) return;
    const loadingDiv = document.createElement('div');
    loadingDiv.className = 'loading-narrative';
    loadingDiv.innerHTML = '<div class="d20-spinner"></div><span class="loading-text">Thinking...</span>';
    storyContent.appendChild(loadingDiv);
    storyContent.scrollTop = storyContent.scrollHeight;
}

function showStateChanges(data) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    const badges = [];
    if (data.gold_delta > 0) badges.push('<span class="sc-badge sc-gold">+' + data.gold_delta + ' gold</span>');
    if (data.gold_delta < 0) badges.push('<span class="sc-badge sc-gold-loss">' + data.gold_delta + ' gold</span>');
    if (data.xp_delta > 0) badges.push('<span class="sc-badge sc-xp">+' + data.xp_delta + ' XP</span>');
    if (data.hp_delta > 0) badges.push('<span class="sc-badge sc-heal">+' + data.hp_delta + ' HP</span>');
    if (data.hp_delta < 0) badges.push('<span class="sc-badge sc-damage">' + data.hp_delta + ' HP</span>');
    if (data.level_up) badges.push('<span class="sc-badge sc-level">LEVEL UP!</span>');
    if (data.items_gained) data.items_gained.forEach(function(i) { badges.push('<span class="sc-badge sc-item-gain">+' + escapeHtml(i) + '</span>'); });
    if (data.items_lost) data.items_lost.forEach(function(i) { badges.push('<span class="sc-badge sc-item-loss">-' + escapeHtml(i) + '</span>'); });
    if (badges.length === 0) return;
    var div = document.createElement('div');
    div.className = 'state-changes';
    div.innerHTML = badges.join(' ');
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

// ============================================================================
// SHOP PANEL
// ============================================================================

var shopState = null; // current shop data from server

function showShopPanel(msg) {
    // Remove loading spinner
    var loadingEl = document.querySelector('.loading-narrative');
    if (loadingEl) loadingEl.remove();

    shopState = msg;
    var existing = document.querySelector('.shop-modal');
    if (existing) existing.remove();

    var modal = document.createElement('div');
    modal.className = 'shop-modal';

    var header = '<div class="shop-header">' +
        '<h2>' + escapeHtml(msg.shop_name) + '</h2>' +
        '<div class="shop-meta">' +
            '<span class="shop-tier">Tier ' + msg.tier + '</span>' +
            '<span class="shop-gold">\u{1FA99} ' + (msg.player_gold || 0) + ' gold</span>' +
        '</div>' +
        '<button class="shop-close">\u2715</button>' +
    '</div>';

    var tabs = '<div class="shop-tabs">' +
        '<button class="shop-tab active" data-tab="buy">Buy</button>' +
        '<button class="shop-tab" data-tab="sell">Sell</button>' +
    '</div>';

    var buyItems = (msg.items || []).sort(function(a, b) { return a.tier - b.tier || a.buy_price - b.buy_price; });
    var buyRows = buyItems.map(function(item) {
        var priceClass = item.price_category === 'above' ? 'price-high' :
                         item.price_category === 'below' ? 'price-low' : 'price-normal';
        var outOfStock = item.current_stock === 0;
        return '<div class="shop-item' + (outOfStock ? ' out-of-stock' : '') + '">' +
            '<div class="shop-item-info">' +
                '<span class="shop-item-name">' + escapeHtml(item.name) + '</span>' +
                '<span class="shop-item-desc">' + escapeHtml(item.description || '') + '</span>' +
            '</div>' +
            '<div class="shop-item-meta">' +
                '<span class="shop-item-tier">T' + item.tier + '</span>' +
                '<span class="shop-item-stock">' + item.current_stock + ' in stock</span>' +
                '<span class="shop-item-price ' + priceClass + '">' + item.buy_price + ' gp</span>' +
                '<button class="shop-buy-btn" data-item-id="' + escapeAttr(item.item_id) + '"' +
                    (outOfStock ? ' disabled' : '') + '>Buy</button>' +
            '</div>' +
        '</div>';
    }).join('');

    // Build sell tab with player inventory
    var invItems = (gameState && gameState.inventory && gameState.inventory.items) || [];
    var sellRows = invItems.map(function(item) {
        var sellPrice = Math.max(Math.floor((item.value_gp || 1) * 0.6), 1);
        return '<div class="shop-item">' +
            '<div class="shop-item-info">' +
                '<span class="shop-item-name">' + escapeHtml(item.name) + '</span>' +
                '<span class="shop-item-desc">' + escapeHtml(item.item_type || '') +
                    (item.quantity > 1 ? ' (x' + item.quantity + ')' : '') + '</span>' +
            '</div>' +
            '<div class="shop-item-meta">' +
                '<span class="shop-item-tier">T' + (item.tier || 0) + '</span>' +
                '<span class="shop-item-price price-normal">~' + sellPrice + ' gp</span>' +
                '<button class="shop-sell-btn" data-item-name="' + escapeAttr(item.name) + '">Sell</button>' +
            '</div>' +
        '</div>';
    }).join('');

    if (sellRows === '') {
        sellRows = '<div class="shop-empty">No items to sell.</div>';
    }

    var body = '<div class="shop-body">' +
        '<div class="shop-tab-content active" data-content="buy">' + buyRows + '</div>' +
        '<div class="shop-tab-content" data-content="sell">' + sellRows + '</div>' +
    '</div>';

    modal.innerHTML = '<div class="shop-panel">' + header + tabs + body + '</div>';
    document.body.appendChild(modal);

    // Event: close
    modal.querySelector('.shop-close').addEventListener('click', closeShopPanel);
    modal.addEventListener('click', function(e) { if (e.target === modal) closeShopPanel(); });

    // Event: tab switching
    modal.querySelectorAll('.shop-tab').forEach(function(tab) {
        tab.addEventListener('click', function() {
            modal.querySelectorAll('.shop-tab').forEach(function(t) { t.classList.remove('active'); });
            modal.querySelectorAll('.shop-tab-content').forEach(function(c) { c.classList.remove('active'); });
            tab.classList.add('active');
            modal.querySelector('.shop-tab-content[data-content="' + tab.dataset.tab + '"]').classList.add('active');
        });
    });

    // Event: buy buttons
    modal.querySelectorAll('.shop-buy-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
            btn.disabled = true;
            btn.textContent = '...';
            ws.send({ type: 'shop_buy', item_id: btn.dataset.itemId, quantity: 1 });
        });
    });

    // Event: sell buttons
    modal.querySelectorAll('.shop-sell-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
            btn.disabled = true;
            btn.textContent = '...';
            ws.send({ type: 'shop_sell', item_name: btn.dataset.itemName });
        });
    });
}

function closeShopPanel() {
    var modal = document.querySelector('.shop-modal');
    if (modal) modal.remove();
    shopState = null;
}

function handleShopBuyResult(msg) {
    if (msg.success) {
        showToast('Bought ' + msg.item_name + ' for ' + msg.price + ' gp');
        if (gameState) gameState.character.gold = msg.gold_remaining;
        // Refresh shop to see updated stock/prices
        ws.send({ type: 'view_shop' });
    } else {
        showToast(msg.error || 'Purchase failed', true);
        // Re-enable buy buttons
        document.querySelectorAll('.shop-buy-btn').forEach(function(btn) {
            btn.disabled = false;
            btn.textContent = 'Buy';
        });
    }
}

function handleShopSellResult(msg) {
    if (msg.success) {
        showToast('Sold ' + msg.item_name + ' for ' + msg.gold_earned + ' gp');
        if (gameState) gameState.character.gold = msg.gold_remaining;
        // Refresh shop to see updated inventory
        ws.send({ type: 'view_shop' });
    } else {
        showToast(msg.error || 'Sale failed', true);
        // Re-enable sell buttons
        document.querySelectorAll('.shop-sell-btn').forEach(function(btn) {
            btn.disabled = false;
            btn.textContent = 'Sell';
        });
    }
}

function showGatherResult(msg) {
    var storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    var loadingEl = storyContent.querySelector('.loading-narrative');
    if (loadingEl) loadingEl.remove();

    var items = (msg.gathered || []).map(function(g) { return g.name || g.id; }).join(', ');
    var biome = msg.biome || 'the wilderness';
    var xp = msg.survival_xp || 0;

    var div = document.createElement('div');
    div.className = 'narrative-block gather-result';
    div.innerHTML = '<p>\u{1FAB5} <strong>Gathering in ' + escapeHtml(biome) + ':</strong> You found ' + escapeHtml(items) + '.</p>' +
        '<p style="font-size:12px;color:var(--text-muted)">+' + xp + ' Survival XP</p>';
    // Insert before choice sections so they stay at the bottom
    var choicesAnchor = storyContent.querySelector('.fixed-choices-section') ||
                        storyContent.querySelector('.choices-separator') ||
                        storyContent.querySelector('.llm-choices-section');
    if (choicesAnchor) {
        storyContent.insertBefore(div, choicesAnchor);
    } else {
        storyContent.appendChild(div);
    }
    storyContent.scrollTop = storyContent.scrollHeight;
    renderFixedActions();
}

function renderFixedActions() {
    var storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    
    var llmSection = storyContent.querySelector('.llm-choices-section');
    
    // Remove existing fixed actions
    document.querySelectorAll('.fixed-choices-section').forEach(function(el) { el.remove(); });

    if (!gameState) return;
    if (gameState.combat && gameState.combat.active) return;
    if (gameState.character && (gameState.character.dead || gameState.character.hp <= 0)) return;

    var buttons = [];

    // Use map_view system exclusively
    var mapView = gameState.map_view;
    if (mapView) {
        if (mapView.directions) {
            mapView.directions.forEach(function(dir) {
                var tierStr = dir.tier && dir.tier !== '?' ? ' (T' + dir.tier + ')' : '';
                var label = dir.direction + ': ' + dir.name + tierStr;
                buttons.push({ label: label, icon: '\u{1F9ED}', action: 'travel_dir:' + dir.direction.toLowerCase(), type: 'travel' });
            });
        }
        var current = mapView.current;
        if (current) {
            if (current.has_town) buttons.push({ label: 'Visit Shop', icon: '\u{1F6D2}', action: 'shop', type: 'shop' });
            if (current.has_dungeon) buttons.push({ label: 'Enter Dungeon', icon: '\u{1F480}', action: 'dungeon', type: 'dungeon' });
            if (current.has_tower) buttons.push({ label: 'Enter ' + (current.tower_name || 'Tower'), icon: '\u{1F3F0}', action: 'tower', type: 'tower' });
            if (current.has_exchange) buttons.push({ label: 'Exchange', icon: '\u{1FA99}', action: 'exchange', type: 'exchange' });
            if (current.has_guild_hall) buttons.push({ label: 'Guild Hall', icon: '\u2694', action: 'guild_hall', type: 'guild_hall' });
            // Gather is always available — every biome has resources
            buttons.push({ label: 'Gather Resources', icon: '\u{1FAB5}', action: 'gather', type: 'gather' });
        }
    }

    // NPCs at location
    var npcsHere = (gameState.npcs || []).filter(function(n) {
        return n.location && gameState.current_scene &&
               n.location.toLowerCase() === gameState.current_scene.location.toLowerCase();
    });
    npcsHere.forEach(function(npc) {
        buttons.push({ label: 'Talk to ' + npc.name, icon: '\u{1F4AC}', action: 'npc:' + npc.name, type: 'npc' });
    });

    if (buttons.length === 0) return;

    var section = document.createElement('div');
    section.className = 'fixed-choices-section';
    section.innerHTML = '<div class="choices-grid">' +
        buttons.map(function(b) {
            return '<button class="choice-btn fixed-choice" data-action="' + b.action + '" data-type="' + b.type + '">' +
                '<span class="fa-icon">' + b.icon + '</span> ' + escapeHtml(b.label) +
                '</button>';
        }).join('') + '</div>';

    if (llmSection) {
        storyContent.insertBefore(section, llmSection);
        if (!storyContent.querySelector('.choices-separator')) {
            var sep = document.createElement('div');
            sep.className = 'choices-separator';
            storyContent.insertBefore(sep, llmSection);
        }
    } else {
        storyContent.appendChild(section);
    }
    storyContent.scrollTop = storyContent.scrollHeight;

    section.querySelectorAll('.fixed-choice').forEach(function(btn) {
        btn.addEventListener('click', function() {
            handleFixedAction(btn.dataset.action, btn.dataset.type);
        });
    });
}

function handleFixedAction(action, type) {
    var mechanical = (type === "gather" || type === "shop");
    if (mechanical) {
        document.querySelectorAll('.fixed-choices-section, .choices-separator').forEach(function(el) { el.remove(); });
    } else {
        document.querySelectorAll('.fixed-choices-section, .llm-choices-section, .choices-container, .choices-separator').forEach(function(el) { el.remove(); });
    }
    if (type === 'travel') {
        var direction = action.replace('travel_dir:', '');
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Travel ' + direction });
    } else if (type === 'shop') {
        ws.send({ type: 'view_shop' });
    } else if (type === 'dungeon') {
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Enter the dungeon' });
    } else if (type === 'tower') {
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Enter the tower' });
    } else if (type === 'exchange') {
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Visit the exchange' });
    } else if (type === 'gather') {
        showLoadingSpinner();
        ws.send({ type: 'gather' });
    } else if (type === 'guild_hall') {
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Visit the guild hall' });
    } else if (type === 'npc') {
        var npcName = action.replace('npc:', '');
        showLoadingSpinner();
        ws.send({ type: 'send_message', content: 'Talk to ' + npcName });
    }
}

init();

// Format narrative text: italicize dialogue (quoted speech)
function formatNarrative(text) {
    const escaped = escapeHtml(text);
    // Wrap text in double quotes or curly quotes in <em> tags
    return escaped.replace(/(\u201c[^\u201d]*\u201d|"[^"]*")/g, '<em class="dialogue">$1</em>');
}
