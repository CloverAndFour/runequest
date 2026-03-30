// Party UI: formation, member list, combat timer, PvP

let partyData = null; // { party_id, leader, members, state, location }
let combatTimer = null;
let combatDeadline = 0;

export function initPartyPanel(container, wsSend) {
    // Party info bar sits above story content when in a party
    const bar = document.createElement('div');
    bar.className = 'party-bar';
    bar.id = 'partyBar';
    bar.style.display = 'none';
    const storyPanel = container.querySelector('.story-panel');
    const storyHeader = container.querySelector('.story-header');
    if (storyPanel && storyHeader) {
        storyHeader.insertAdjacentElement('afterend', bar);
    }
}

function renderPartyBar(wsSend) {
    const bar = document.getElementById('partyBar');
    if (!bar || !partyData) { if (bar) bar.style.display = 'none'; return; }
    bar.style.display = '';

    const isLeader = partyData.leader === (window._rqUsername || '');
    let html = '<div class="pb-members">';
    partyData.members.forEach(m => {
        const hpPct = m.max_hp > 0 ? (Math.max(m.hp, 0) / m.max_hp * 100) : 100;
        const hpClass = hpPct > 50 ? '' : hpPct > 25 ? ' warning' : ' critical';
        const incap = m.incapacitated ? ' incap' : '';
        const leader = m.username === partyData.leader ? ' <span class="pb-leader">L</span>' : '';
        html += '<div class="pb-member' + incap + '" title="' + esc(m.character_name) + '">' +
            '<div class="pb-name">' + esc(m.character_name) + leader + '</div>' +
            '<div class="pb-hp-bar"><div class="pb-hp-fill' + hpClass + '" style="width:' + hpPct + '%"></div></div>' +
            '<div class="pb-hp-text">' + m.hp + '/' + m.max_hp + '</div>' +
            '</div>';
    });
    html += '</div>';

    if (isLeader) {
        html += '<button class="pb-disband" onclick="window._rqPartyLeave()">Leave Party</button>';
    } else {
        html += '<button class="pb-leave" onclick="window._rqPartyLeave()">Leave</button>';
    }

    bar.innerHTML = html;

    window._rqPartyLeave = () => { wsSend({ type: 'leave_party' }); };
}

// Combat timer bar
function renderCombatTimer() {
    let el = document.getElementById('combatTimerBar');
    if (!el) {
        el = document.createElement('div');
        el.className = 'combat-timer-bar';
        el.id = 'combatTimerBar';
        const bar = document.getElementById('partyBar');
        if (bar) bar.appendChild(el);
    }
    if (combatDeadline <= 0) { el.style.display = 'none'; return; }
    el.style.display = '';
    const remaining = Math.max(0, combatDeadline - Date.now());
    const pct = (remaining / 30000) * 100;
    el.innerHTML = '<div class="ct-fill" style="width:' + pct + '%"></div><span class="ct-text">' +
        Math.ceil(remaining / 1000) + 's</span>';
}

function startCombatTimerLoop() {
    if (combatTimer) clearInterval(combatTimer);
    combatTimer = setInterval(renderCombatTimer, 200);
}

function stopCombatTimer() {
    if (combatTimer) { clearInterval(combatTimer); combatTimer = null; }
    combatDeadline = 0;
    const el = document.getElementById('combatTimerBar');
    if (el) el.style.display = 'none';
}

// --- Public handlers ---

export function handlePartyInfo(msg, wsSend) {
    partyData = msg;
    renderPartyBar(wsSend);
}

export function handlePartyMemberJoined(msg, wsSend) {
    if (!partyData) return;
    if (!partyData.members.find(m => m.username === msg.username)) {
        partyData.members.push({
            username: msg.username,
            character_name: msg.character_name,
            character_class: msg.character_class,
            hp: 0, max_hp: 0, ready: false, incapacitated: false,
        });
    }
    renderPartyBar(wsSend);
}

export function handlePartyMemberLeft(msg, wsSend) {
    if (!partyData) return;
    partyData.members = partyData.members.filter(m => m.username !== msg.username);
    if (partyData.members.length <= 1) {
        partyData = null;
        stopCombatTimer();
    }
    renderPartyBar(wsSend);
}

export function handlePartyDisbanded(msg) {
    partyData = null;
    stopCombatTimer();
    const bar = document.getElementById('partyBar');
    if (bar) bar.style.display = 'none';
    showToast('Party disbanded: ' + (msg.reason || ''));
}

export function handlePartyCombatStarted(msg, wsSend) {
    // Show combat info in story panel
    const storyContent = document.querySelector('.story-content');
    if (storyContent) {
        const div = document.createElement('div');
        div.className = 'narrative-block';
        div.innerHTML = '<strong style="color:var(--accent-red)">\u2694 Party Combat!</strong> ' +
            msg.enemies.map(e => esc(e.name)).join(', ');
        storyContent.appendChild(div);
        storyContent.scrollTop = storyContent.scrollHeight;
    }
}

export function handlePartyCombatPhaseStart(msg, wsSend) {
    combatDeadline = Date.now() + msg.deadline_ms;
    startCombatTimerLoop();

    // Show action buttons
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;

    // Remove previous action UI
    storyContent.querySelector('.party-combat-actions')?.remove();

    const div = document.createElement('div');
    div.className = 'party-combat-actions';

    let html = '<div class="pca-header">Round ' + msg.round + ' - Choose your action (' +
        Math.ceil(msg.deadline_ms / 1000) + 's)</div>';
    html += '<div class="pca-hp">';
    (msg.party_hp || []).forEach(p => {
        const pct = p.max_hp > 0 ? (Math.max(p.hp, 0) / p.max_hp * 100) : 0;
        html += '<span class="pca-hp-entry">' + esc(p.character_name) + ': ' + p.hp + '/' + p.max_hp + '</span> ';
    });
    html += '</div>';
    html += '<div class="pca-enemies">';
    (msg.enemies || []).forEach(e => {
        if (e.alive) html += '<span class="pca-enemy">' + esc(e.name) + ' (' + e.hp + '/' + e.max_hp + ')</span> ';
    });
    html += '</div>';
    html += '<div class="pca-buttons">';
    (msg.your_available_actions || []).forEach(a => {
        if (a.enabled) {
            html += '<button class="stone-btn pca-btn" data-action="' + esc(a.id) + '">' +
                esc(a.name) + '</button>';
        }
    });
    html += '<button class="stone-btn pca-btn pca-ready" data-action="__ready__">Ready (Dodge)</button>';
    html += '</div>';

    div.innerHTML = html;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;

    div.querySelectorAll('.pca-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const aid = btn.dataset.action;
            if (aid === '__ready__') {
                wsSend({ type: 'party_combat_ready' });
            } else {
                // For attack, pick first living enemy as target
                const firstEnemy = (msg.enemies || []).find(e => e.alive);
                wsSend({ type: 'party_combat_action', action_id: aid, target: firstEnemy?.name || null });
            }
            div.remove();
            stopCombatTimer();
        });
    });
}

export function handlePartyCombatResolution(msg) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    (msg.results || []).forEach(r => {
        const div = document.createElement('div');
        div.className = 'combat-action-log ' + (r.hit === true ? 'hit' : r.hit === false ? 'miss' : 'neutral');
        div.textContent = r.description;
        storyContent.appendChild(div);
    });
    storyContent.scrollTop = storyContent.scrollHeight;
}

export function handlePartyCombatEnemyPhase(msg) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    (msg.results || []).forEach(r => {
        const div = document.createElement('div');
        div.className = 'combat-action-log enemy ' + (r.hit ? 'hit' : 'miss');
        div.textContent = r.enemy_name + ' attacks ' + r.target + ' — ' +
            (r.hit ? 'HIT for ' + r.damage + ' damage!' : 'MISS');
        storyContent.appendChild(div);
    });
    storyContent.scrollTop = storyContent.scrollHeight;
}

export function handlePartyCombatEnded(msg, wsSend) {
    stopCombatTimer();
    const storyContent = document.querySelector('.story-content');
    if (storyContent) {
        const div = document.createElement('div');
        div.className = 'narrative-block';
        if (msg.victory) {
            div.innerHTML = '<strong style="color:var(--accent-green)">\u2713 Victory!</strong> +' +
                msg.xp_per_member + ' XP each';
        } else {
            div.innerHTML = '<strong style="color:var(--accent-red)">\u2717 Defeated...</strong>';
        }
        storyContent.appendChild(div);
        storyContent.scrollTop = storyContent.scrollHeight;
    }
    // Refresh party info
    if (wsSend) wsSend({ type: 'get_party_info' });
}

export function handlePartyTrapResults(msg) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    (msg.results || []).forEach(r => {
        const div = document.createElement('div');
        div.className = 'condition-effects';
        let text = r.character_name + ': ';
        if (r.detected) text += 'detected the trap! ';
        else {
            text += r.saved ? 'partially dodged' : 'caught in the trap';
            text += ' (' + r.damage + ' damage)';
            if (r.condition) text += ' [' + r.condition + ']';
        }
        div.textContent = text;
        storyContent.appendChild(div);
    });
    storyContent.scrollTop = storyContent.scrollHeight;
}

// PvP
export function handlePvpChallengeReceived(msg, wsSend) {
    const accept = confirm(msg.character_name + ' challenges you to a duel! Accept?');
    if (accept) {
        wsSend({ type: 'accept_pvp_challenge', challenger: msg.challenger });
    } else {
        wsSend({ type: 'decline_pvp_challenge', challenger: msg.challenger });
    }
}

export function handlePvpStarted(msg) {
    const storyContent = document.querySelector('.story-content');
    if (storyContent) {
        const div = document.createElement('div');
        div.className = 'narrative-block';
        div.innerHTML = '<strong style="color:var(--accent-red)">\u2694 PvP Duel vs ' +
            esc(msg.opponent_name) + '!</strong>';
        storyContent.appendChild(div);
        storyContent.scrollTop = storyContent.scrollHeight;
    }
}

export function handlePvpEnded(msg) {
    const storyContent = document.querySelector('.story-content');
    if (storyContent) {
        const div = document.createElement('div');
        div.className = 'narrative-block';
        div.innerHTML = msg.victory
            ? '<strong style="color:var(--accent-green)">\u2713 You won the duel!</strong>' +
              (msg.criminal ? ' <span style="color:var(--accent-red)">[CRIMINAL]</span>' : '')
            : '<strong style="color:var(--accent-red)">\u2717 You were defeated...</strong>';
        storyContent.appendChild(div);
        storyContent.scrollTop = storyContent.scrollHeight;
    }
}

export function handleCriminalStatus(msg) {
    // Show/hide criminal badge on player info
    const badge = document.getElementById('criminalBadge_' + msg.username);
    if (badge) badge.style.display = msg.is_criminal ? '' : 'none';
}

export function getPartyData() { return partyData; }

function showToast(message) {
    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 4000);
}

function esc(str) {
    const d = document.createElement('div');
    d.textContent = str || '';
    return d.innerHTML;
}
