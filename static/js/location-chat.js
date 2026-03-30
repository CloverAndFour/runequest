// Location chat - public messages at the current world location

let locationMessages = [];
let locationPlayers = [];
let currentLocation = '';
let locationChatVisible = false;

export function initLocationChat(container, wsSend) {
    // Create location chat toggle in story header
    const header = container.querySelector('.story-header');
    if (!header) return;

    const toggle = document.createElement('button');
    toggle.className = 'location-chat-toggle';
    toggle.id = 'locationChatToggle';
    toggle.innerHTML = '<span class="lct-icon">\u{1F4E2}</span><span class="lct-badge" id="locChatBadge" style="display:none;">0</span>';
    toggle.title = 'Location Chat';

    // Insert after the options button
    const optionsBtn = header.querySelector('#optionsBtn');
    if (optionsBtn) {
        optionsBtn.insertAdjacentElement('beforebegin', toggle);
    } else {
        header.appendChild(toggle);
    }

    // Create chat overlay
    const overlay = document.createElement('div');
    overlay.className = 'location-chat-overlay';
    overlay.id = 'locationChatOverlay';
    overlay.style.display = 'none';
    overlay.innerHTML = `
        <div class="lco-header">
            <span class="lco-title" id="lcoTitle">Location Chat</span>
            <span class="lco-players" id="lcoPlayers">0 players here</span>
        </div>
        <div class="lco-messages" id="lcoMessages"></div>
        <div class="lco-input">
            <input type="text" id="locChatInput" placeholder="Say something..." autocomplete="off">
            <button class="stone-btn" id="locChatSend">Send</button>
        </div>
    `;

    const storyPanel = container.querySelector('.story-panel');
    if (storyPanel) storyPanel.appendChild(overlay);

    toggle.addEventListener('click', () => {
        locationChatVisible = !locationChatVisible;
        overlay.style.display = locationChatVisible ? 'flex' : 'none';
        toggle.classList.toggle('active', locationChatVisible);
        if (locationChatVisible) {
            wsSend({ type: 'get_location_players' });
            document.getElementById('locChatInput')?.focus();
        }
    });

    overlay.querySelector('#locChatSend')?.addEventListener('click', () => {
        sendLocationMessage(wsSend);
    });
    overlay.querySelector('#locChatInput')?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') sendLocationMessage(wsSend);
    });

    // Request players on load
    wsSend({ type: 'get_location_players' });
}

function sendLocationMessage(wsSend) {
    const input = document.getElementById('locChatInput');
    if (!input) return;
    const text = input.value.trim();
    if (!text) return;
    wsSend({ type: 'send_location_chat', text });
    input.value = '';
}

function renderMessages() {
    const el = document.getElementById('lcoMessages');
    if (!el) return;

    const fiveMinAgo = Date.now() - 5 * 60 * 1000;
    const recentMessages = locationMessages.filter(m => {
        if (!m.ts) return true;
        return new Date(m.ts).getTime() > fiveMinAgo;
    });

    el.innerHTML = recentMessages.map(m => {
        const time = new Date(m.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        const isSystem = m.character_name && m.character_name.startsWith('[');
        const msgClass = isSystem ? 'lco-msg lco-msg-system' : 'lco-msg';
        return `<div class="${msgClass}">
            <span class="lco-msg-name">${esc(m.character_name || m.from)}</span>
            <span class="lco-msg-text">${esc(m.text)}</span>
            <span class="lco-msg-time">${time}</span>
        </div>`;
    }).join('');

    el.scrollTop = el.scrollHeight;
}

function updatePlayerCount() {
    const el = document.getElementById('lcoPlayers');
    if (el) {
        const n = locationPlayers.length;
        el.textContent = n === 0 ? 'No other players' : `${n} other player${n > 1 ? 's' : ''} here`;
    }
    const title = document.getElementById('lcoTitle');
    if (title && currentLocation) {
        title.textContent = currentLocation;
    }
}

// --- Public handlers ---

export function handleLocationChat(msg) {
    locationMessages.push({
        from: msg.from,
        character_name: msg.character_name,
        text: msg.text,
        ts: msg.ts,
    });
    // Keep last 50
    if (locationMessages.length > 50) locationMessages.shift();
    renderMessages();

    // Badge if panel hidden
    if (!locationChatVisible) {
        const badge = document.getElementById('locChatBadge');
        if (badge) {
            const n = parseInt(badge.textContent || '0') + 1;
            badge.textContent = n;
            badge.style.display = '';
        }
    }
}

export function handleLocationPresence(msg) {
    currentLocation = msg.location;
    locationPlayers = msg.players || [];
    updatePlayerCount();

    // Dispatch event for world map to show player counts
    document.dispatchEvent(new CustomEvent('location-players-update', {
        detail: { location: msg.location, players: msg.players }
    }));
}

export function handleLocationChatHistory(msg) {
    locationMessages = (msg.messages || []).map(m => ({
        from: m.from,
        character_name: m.character_name,
        text: m.text,
        ts: m.ts,
    }));
    currentLocation = msg.location;
    renderMessages();
}

export function onLocationChange(newLocation) {
    // Clear messages when location changes
    if (newLocation !== currentLocation) {
        locationMessages = [];
        locationPlayers = [];
        currentLocation = newLocation;
        renderMessages();
        updatePlayerCount();
    }
}

export function getLocationPlayers() {
    return locationPlayers;
}

function esc(str) {
    const d = document.createElement('div');
    d.textContent = str || '';
    return d.innerHTML;
}
