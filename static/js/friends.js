// Friends panel - friend list, requests, and chat

let friendsData = { friends: [], incoming_requests: [], outgoing_requests: [] };
let myTag = '';
let activeChat = null; // username of friend we're chatting with
let chatMessages = {}; // { username: [msg, ...] }
let panelOpen = false;
let unreadCounts = {}; // { username: count }

export function initFriendsPanel(container, wsSend) {
    // Insert toggle button and panel into the layout
    const panel = document.createElement('div');
    panel.className = 'friends-panel collapsed';
    panel.id = 'friendsPanel';
    panel.innerHTML = buildPanelHtml();

    const toggle = document.createElement('button');
    toggle.className = 'friends-toggle';
    toggle.id = 'friendsToggle';
    toggle.innerHTML = '<span class="ft-icon">\u{1F465}</span><span class="ft-badge" id="friendsBadge" style="display:none;">0</span>';
    toggle.title = 'Friends';

    // Insert panel before story-panel
    const storyPanel = container.querySelector('.story-panel');
    if (storyPanel) {
        container.insertBefore(panel, storyPanel);
    }

    // Insert toggle into the story header
    const header = container.querySelector('.story-header');
    if (header) {
        header.insertBefore(toggle, header.firstChild);
    }

    toggle.addEventListener('click', () => {
        panelOpen = !panelOpen;
        panel.classList.toggle('collapsed', !panelOpen);
        container.classList.toggle('friends-open', panelOpen);
        toggle.classList.toggle('active', panelOpen);
        if (panelOpen) {
            wsSend({ type: 'get_friends' });
            wsSend({ type: 'get_friend_code' });
        }
    });

    // Wire up add friend
    panel.addEventListener('click', (e) => {
        const btn = e.target.closest('[data-action]');
        if (!btn) return;
        const action = btn.dataset.action;

        if (action === 'add-friend') {
            showAddFriendModal(wsSend);
        } else if (action === 'accept') {
            wsSend({ type: 'accept_friend_request', username: btn.dataset.username });
        } else if (action === 'decline') {
            wsSend({ type: 'decline_friend_request', username: btn.dataset.username });
        } else if (action === 'chat') {
            openChat(btn.dataset.username, wsSend);
        } else if (action === 'close-chat') {
            closeChat();
        } else if (action === 'remove') {
            if (confirm('Remove ' + btn.dataset.username + ' from friends?')) {
                wsSend({ type: 'remove_friend', username: btn.dataset.username });
            }
        }
    });

    // Chat send
    panel.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && e.target.id === 'chatInput') {
            sendChatMessage(wsSend);
        }
    });
    panel.addEventListener('click', (e) => {
        if (e.target.id === 'chatSendBtn') {
            sendChatMessage(wsSend);
        }
    });

    // Request initial data
    wsSend({ type: 'get_friend_code' });
    wsSend({ type: 'get_friends' });
}

function buildPanelHtml() {
    return `
        <div class="fp-header">
            <span class="fp-title">Friends</span>
            <button class="fp-add-btn" data-action="add-friend" title="Add Friend">+</button>
        </div>
        <div class="fp-tag" id="fpTag" title="Click to copy your friend code"></div>
        <div class="fp-requests" id="fpRequests"></div>
        <div class="fp-list" id="fpList"></div>
        <div class="fp-chat" id="fpChat" style="display:none;">
            <div class="fpc-header">
                <span class="fpc-name" id="fpcName"></span>
                <button class="fpc-close" data-action="close-chat">&times;</button>
            </div>
            <div class="fpc-messages" id="fpcMessages"></div>
            <div class="fpc-input">
                <input type="text" id="chatInput" placeholder="Message..." autocomplete="off">
                <button id="chatSendBtn" class="stone-btn">Send</button>
            </div>
        </div>
    `;
}

function renderFriendsList() {
    const listEl = document.getElementById('fpList');
    if (!listEl) return;

    if (friendsData.friends.length === 0) {
        listEl.innerHTML = '<div class="fp-empty">No friends yet. Add someone!</div>';
        return;
    }

    // Sort: online first, then alphabetical
    const sorted = [...friendsData.friends].sort((a, b) => {
        if (a.online && !b.online) return -1;
        if (!a.online && b.online) return 1;
        return a.username.localeCompare(b.username);
    });

    listEl.innerHTML = sorted.map(f => {
        const unread = unreadCounts[f.username] || 0;
        const badge = unread > 0 ? `<span class="fp-unread">${unread}</span>` : '';
        const charInfo = f.character_name
            ? `<div class="fp-char">${esc(f.character_name)}${f.character_class ? ', ' + esc(f.character_class) : ''}</div>`
            : '';
        const locInfo = f.online && f.location
            ? `<div class="fp-loc">${esc(f.location)}</div>`
            : '';
        return `<div class="fp-friend ${f.online ? 'online' : 'offline'}" data-action="chat" data-username="${esc(f.username)}">
            <div class="fp-status-dot ${f.online ? 'on' : 'off'}"></div>
            <div class="fp-info">
                <div class="fp-name">${esc(f.username)}<span class="fp-code">#${esc(f.friend_code)}</span>${badge}</div>
                ${charInfo}
                ${locInfo}
            </div>
            <button class="fp-remove-btn" data-action="remove" data-username="${esc(f.username)}" title="Remove">&times;</button>
        </div>`;
    }).join('');
}

function renderRequests() {
    const el = document.getElementById('fpRequests');
    if (!el) return;

    const reqs = friendsData.incoming_requests || [];
    if (reqs.length === 0) {
        el.innerHTML = '';
        return;
    }

    el.innerHTML = '<div class="fp-section-title">Friend Requests</div>' +
        reqs.map(r => `<div class="fp-request">
            <span class="fp-req-name">${esc(r.username)}#${esc(r.friend_code)}</span>
            <button class="fp-accept-btn" data-action="accept" data-username="${esc(r.username)}">Accept</button>
            <button class="fp-decline-btn" data-action="decline" data-username="${esc(r.username)}">Decline</button>
        </div>`).join('');
}

function updateBadge() {
    const badge = document.getElementById('friendsBadge');
    if (!badge) return;
    const total = (friendsData.incoming_requests || []).length +
        Object.values(unreadCounts).reduce((a, b) => a + b, 0);
    badge.textContent = total;
    badge.style.display = total > 0 ? '' : 'none';
}

function showAddFriendModal(wsSend) {
    const existing = document.querySelector('.add-friend-modal');
    if (existing) { existing.remove(); return; }

    const modal = document.createElement('div');
    modal.className = 'add-friend-modal';
    modal.innerHTML = `
        <div class="afm-content">
            <h3>Add Friend</h3>
            <p style="font-size:12px;color:var(--text-muted);margin-bottom:12px;">Enter their tag (e.g. player#123456)</p>
            <input type="text" id="addFriendInput" placeholder="username#000000" autocomplete="off">
            <div class="afm-buttons">
                <button class="stone-btn" id="addFriendSend">Send Request</button>
                <button class="stone-btn" id="addFriendCancel">Cancel</button>
            </div>
            <div class="afm-result" id="addFriendResult"></div>
        </div>
    `;
    document.body.appendChild(modal);

    document.getElementById('addFriendSend').addEventListener('click', () => {
        const tag = document.getElementById('addFriendInput').value.trim();
        if (tag) wsSend({ type: 'send_friend_request', friend_tag: tag });
    });
    document.getElementById('addFriendCancel').addEventListener('click', () => modal.remove());
    modal.addEventListener('click', (e) => { if (e.target === modal) modal.remove(); });
    document.getElementById('addFriendInput').focus();
}

function openChat(username, wsSend) {
    activeChat = username;
    unreadCounts[username] = 0;
    updateBadge();

    const chatEl = document.getElementById('fpChat');
    const listEl = document.getElementById('fpList');
    if (chatEl) chatEl.style.display = 'flex';
    if (listEl) listEl.style.display = 'none';

    document.getElementById('fpcName').textContent = username;

    // Load history
    wsSend({ type: 'get_chat_history', friend: username, limit: 50 });

    // Render what we have
    renderChatMessages();
    document.getElementById('chatInput')?.focus();
}

function closeChat() {
    activeChat = null;
    const chatEl = document.getElementById('fpChat');
    const listEl = document.getElementById('fpList');
    if (chatEl) chatEl.style.display = 'none';
    if (listEl) listEl.style.display = '';
}

function renderChatMessages() {
    const el = document.getElementById('fpcMessages');
    if (!el || !activeChat) return;

    const msgs = chatMessages[activeChat] || [];
    el.innerHTML = msgs.map(m => {
        const isMe = m.from !== activeChat;
        const time = new Date(m.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        return `<div class="fpc-msg ${isMe ? 'me' : 'them'}">
            <span class="fpc-msg-text">${esc(m.text)}</span>
            <span class="fpc-msg-time">${time}</span>
        </div>`;
    }).join('');

    el.scrollTop = el.scrollHeight;
}

function sendChatMessage(wsSend) {
    const input = document.getElementById('chatInput');
    if (!input || !activeChat) return;
    const text = input.value.trim();
    if (!text) return;
    wsSend({ type: 'send_chat', to: activeChat, text });
    input.value = '';
}

// --- Public handlers for server messages ---

export function handleFriendCode(msg) {
    myTag = msg.tag;
    const el = document.getElementById('fpTag');
    if (el) {
        el.textContent = myTag;
        el.onclick = () => {
            navigator.clipboard.writeText(myTag);
            el.textContent = 'Copied!';
            setTimeout(() => { el.textContent = myTag; }, 1500);
        };
    }
}

export function handleFriendsList(msg) {
    friendsData = msg;
    renderFriendsList();
    renderRequests();
    updateBadge();
}

export function handleFriendPresence(msg) {
    // Update the friend in our local data
    const existing = friendsData.friends.find(f => f.username === msg.username);
    if (existing) {
        existing.online = msg.online;
        existing.character_name = msg.character_name;
        existing.character_class = msg.character_class;
        existing.location = msg.location;
        renderFriendsList();
    }

    // Dispatch custom event for world map markers
    document.dispatchEvent(new CustomEvent('friend-presence-update', { detail: friendsData.friends }));
}

export function handleFriendRequestReceived(msg) {
    // Add to incoming requests if not already there
    if (!friendsData.incoming_requests.find(r => r.username === msg.from_username)) {
        friendsData.incoming_requests.push({
            username: msg.from_username,
            friend_code: msg.from_tag.split('#')[1] || '',
        });
    }
    renderRequests();
    updateBadge();
}

export function handleFriendRequestAccepted(msg) {
    // Add them to friends list
    if (!friendsData.friends.find(f => f.username === msg.username)) {
        friendsData.friends.push({
            username: msg.username,
            friend_code: msg.friend_code,
            online: true,
            character_name: null,
            character_class: null,
            location: null,
        });
    }
    // Remove from outgoing
    friendsData.outgoing_requests = friendsData.outgoing_requests.filter(u => u !== msg.username);
    renderFriendsList();
    updateBadge();
}

export function handleFriendRequestSent(msg) {
    const result = document.getElementById('addFriendResult');
    if (result) {
        result.textContent = msg.message;
        result.style.color = msg.success ? 'var(--accent-green)' : 'var(--accent-red)';
        if (msg.success) {
            setTimeout(() => {
                document.querySelector('.add-friend-modal')?.remove();
            }, 1500);
        }
    }
}

export function handleFriendChat(msg) {
    if (!chatMessages[msg.from]) chatMessages[msg.from] = [];

    // Avoid duplicates (echo from server)
    const msgs = chatMessages[msg.from];
    const isDup = msgs.length > 0 &&
        msgs[msgs.length - 1].text === msg.text &&
        msgs[msgs.length - 1].from === msg.from &&
        Math.abs(new Date(msgs[msgs.length - 1].ts) - new Date(msg.ts)) < 2000;

    if (!isDup) {
        chatMessages[msg.from].push(msg);
    }

    if (activeChat === msg.from) {
        renderChatMessages();
    } else {
        // Increment unread
        unreadCounts[msg.from] = (unreadCounts[msg.from] || 0) + 1;
        updateBadge();
        renderFriendsList();
    }
}

export function handleFriendChatSent(msg) {
    // Our own message echoed back: store under recipient
    // msg.from is us, msg.to might not be in message, but we know activeChat
    const friend = activeChat;
    if (!friend) return;
    if (!chatMessages[friend]) chatMessages[friend] = [];

    const msgs = chatMessages[friend];
    const isDup = msgs.length > 0 &&
        msgs[msgs.length - 1].text === msg.text &&
        msgs[msgs.length - 1].from === msg.from &&
        Math.abs(new Date(msgs[msgs.length - 1].ts) - new Date(msg.ts)) < 2000;

    if (!isDup) {
        chatMessages[friend].push(msg);
    }
    renderChatMessages();
}

export function handleFriendChatHistory(msg) {
    chatMessages[msg.friend] = msg.messages || [];
    if (activeChat === msg.friend) {
        renderChatMessages();
    }
}

export function getFriendsForMap() {
    return friendsData.friends || [];
}

function esc(str) {
    const d = document.createElement('div');
    d.textContent = str || '';
    return d.innerHTML;
}
