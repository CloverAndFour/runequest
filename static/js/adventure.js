// Adventure screen renderer and info panel

export function renderAdventure(container, state, handlers) {
    if (!state) {
        container.innerHTML = '<div class="loading">Loading adventure</div>';
        return;
    }

    const c = state.character || {};
    const stats = c.stats || {};

    container.innerHTML = `
    <div class="adventure-layout">
        <div class="story-panel">
            <div class="story-header">
                <button class="btn-back" id="advBackBtn" style="position:static;">&larr;</button>
                <h2>${escapeHtml(state.name || 'Adventure')}</h2>
                <span class="scene-location" id="sceneLocation">${escapeHtml(state.current_scene?.location || 'Unknown')}</span>
            </div>
            <div class="story-content" id="storyContent">
                <div class="narrative-block" style="color: var(--text-muted); font-style: italic;">
                    Your adventure is loading...
                </div>
            </div>
        </div>
        <div class="info-panel">
            <div class="info-tabs">
                <button class="info-tab active" data-tab="stats">Stats</button>
                <button class="info-tab" data-tab="inventory">Items</button>
                <button class="info-tab" data-tab="abilities">Skills</button>
                <button class="info-tab" data-tab="quests">Quests</button>
            </div>
            <div class="info-content" id="infoContent"></div>
        </div>
    </div>`;

    // Tab switching
    let activeTab = 'stats';
    container.querySelectorAll('.info-tab').forEach(tab => {
        tab.addEventListener('click', () => {
            container.querySelectorAll('.info-tab').forEach(t => t.classList.remove('active'));
            tab.classList.add('active');
            activeTab = tab.dataset.tab;
            renderTab(activeTab, state);
        });
    });

    // Back button
    document.getElementById('advBackBtn')?.addEventListener('click', handlers.onBack);

    // State update listener
    const stateHandler = (e) => {
        Object.assign(state, e.detail);
        renderTab(activeTab, state);
        // Update scene location
        const loc = document.getElementById('sceneLocation');
        if (loc && state.current_scene) {
            loc.textContent = state.current_scene.location || 'Unknown';
        }
    };
    document.addEventListener('state-update', stateHandler);

    // Initial render
    renderTab(activeTab, state);
}

function renderTab(tab, state) {
    const el = document.getElementById('infoContent');
    if (!el) return;

    switch (tab) {
        case 'stats':
            renderStats(el, state);
            break;
        case 'inventory':
            renderInventory(el, state);
            break;
        case 'abilities':
            renderAbilities(el, state);
            break;
        case 'quests':
            renderQuests(el, state);
            break;
    }
}

function renderStats(el, state) {
    const c = state.character || {};
    const s = c.stats || {};
    const hpPct = c.max_hp > 0 ? (Math.max(c.hp, 0) / c.max_hp * 100) : 0;
    const hpClass = hpPct > 50 ? 'hp' : hpPct > 25 ? 'hp warning' : 'hp critical';
    const xpNext = getXpNext(c.level || 1);
    const xpPct = xpNext > 0 ? ((c.xp || 0) / xpNext * 100) : 0;

    el.innerHTML = `
        <div class="char-name">${escapeHtml(c.name || 'Unknown')}</div>
        <div class="char-class">${escapeHtml(c.race || '?')} ${escapeHtml(c.class || '?')} &middot; Level ${c.level || 1}</div>

        <div class="hp-bar-container">
            <div class="bar-label"><span>HP</span><span class="value">${c.hp || 0} / ${c.max_hp || 0}</span></div>
            <div class="bar-track"><div class="bar-fill ${hpClass}" style="width: ${hpPct}%"></div></div>
        </div>

        <div class="xp-bar-container">
            <div class="bar-label"><span>XP</span><span class="value">${c.xp || 0} / ${xpNext}</span></div>
            <div class="bar-track"><div class="bar-fill xp" style="width: ${Math.min(xpPct, 100)}%"></div></div>
        </div>

        <div style="text-align:center; font-size:12px; color:var(--text-muted); margin:8px 0;">
            AC: <span style="color:var(--text-gold)">${c.ac || 10}</span> &middot;
            Proficiency: <span style="color:var(--text-gold)">+${getProficiency(c.level || 1)}</span>
        </div>

        <div class="stats-grid">
            ${renderStatBox('STR', s.strength)}
            ${renderStatBox('DEX', s.dexterity)}
            ${renderStatBox('CON', s.constitution)}
            ${renderStatBox('INT', s.intelligence)}
            ${renderStatBox('WIS', s.wisdom)}
            ${renderStatBox('CHA', s.charisma)}
        </div>

        ${c.conditions && c.conditions.length > 0 ? `
            <div style="margin-top:12px; font-size:12px; color:var(--accent-red);">
                Conditions: ${c.conditions.join(', ')}
            </div>
        ` : ''}
    `;
}

function renderStatBox(name, value) {
    const v = value || 10;
    const mod = Math.floor((v - 10) / 2);
    const modStr = mod >= 0 ? `+${mod}` : `${mod}`;
    return `<div class="stat-box">
        <div class="stat-name">${name}</div>
        <div class="stat-value">${v}</div>
        <div class="stat-mod">${modStr}</div>
    </div>`;
}

function renderInventory(el, state) {
    const inv = state.inventory || { items: [] };
    if (inv.items.length === 0) {
        el.innerHTML = '<div class="empty-state">Your pack is empty.</div>';
        return;
    }

    const typeIcons = {
        weapon: '\u2694',
        armor: '\u{1F6E1}',
        potion: '\u{1F9EA}',
        scroll: '\u{1F4DC}',
        misc: '\u{1F4E6}',
    };

    let html = '<ul class="item-list">';
    inv.items.forEach(item => {
        const icon = typeIcons[item.item_type] || '\u{1F4E6}';
        html += `<li class="item-entry" title="${escapeAttr(item.description || '')}">
            <span class="item-icon">${icon}</span>
            <span class="item-name">${escapeHtml(item.name)}</span>
            <span class="item-type">${item.item_type || 'misc'}</span>
        </li>`;
    });
    html += '</ul>';

    const totalWeight = inv.items.reduce((sum, i) => sum + (i.weight || 0), 0);
    html += `<div style="text-align:center; font-size:11px; color:var(--text-muted); margin-top:8px;">
        Total weight: ${totalWeight.toFixed(1)} lbs
    </div>`;

    el.innerHTML = html;
}

function renderAbilities(el, state) {
    const abilities = state.abilities || [];
    const slots = state.spell_slots || {};

    let html = '';

    if (abilities.length === 0) {
        html = '<div class="empty-state">No abilities yet.</div>';
    } else {
        abilities.forEach(a => {
            html += `<div style="padding:8px 0; border-bottom:1px solid rgba(74,58,42,0.3);">
                <div style="color:var(--text-gold); font-family:var(--font-medieval);">${escapeHtml(a.name)}</div>
                <div style="font-size:12px; color:var(--text-muted); margin-top:2px;">${escapeHtml(a.description)}</div>
                ${a.uses_per_rest != null ? `<div style="font-size:11px; color:var(--text-light); margin-top:2px;">Uses: ${a.uses_remaining ?? '?'}/${a.uses_per_rest}</div>` : ''}
            </div>`;
        });
    }

    // Spell slots
    if (slots.level_1 > 0 || slots.level_2 > 0 || slots.level_3 > 0) {
        html += '<div style="margin-top:16px; font-size:12px; color:var(--text-gold);">Spell Slots</div>';
        if (slots.level_1 > 0) html += renderSlotRow('1st', slots.level_1, slots.level_1_used || 0);
        if (slots.level_2 > 0) html += renderSlotRow('2nd', slots.level_2, slots.level_2_used || 0);
        if (slots.level_3 > 0) html += renderSlotRow('3rd', slots.level_3, slots.level_3_used || 0);
    }

    el.innerHTML = html;
}

function renderSlotRow(label, total, used) {
    let dots = '';
    for (let i = 0; i < total; i++) {
        const filled = i < (total - used);
        dots += `<span style="display:inline-block;width:14px;height:14px;border-radius:50%;border:1px solid var(--border-gold);background:${filled ? 'var(--text-gold)' : 'transparent'};margin-right:3px;"></span>`;
    }
    return `<div style="padding:4px 0;font-size:12px;color:var(--text-muted);">${label}: ${dots}</div>`;
}

function renderQuests(el, state) {
    const quests = state.quest_log || [];
    if (quests.length === 0) {
        el.innerHTML = '<div class="empty-state">No quests yet.</div>';
        return;
    }

    let html = '';
    quests.forEach(q => {
        html += `<div class="quest-entry">
            <div class="quest-name ${q.completed ? 'completed' : ''}">${escapeHtml(q.name)}</div>
            <div class="quest-desc">${escapeHtml(q.description)}</div>
        </div>`;
    });
    el.innerHTML = html;
}

function getXpNext(level) {
    const thresholds = [0, 300, 900, 2700, 6500, 14000, 23000, 34000, 48000, 64000];
    return level < thresholds.length ? thresholds[level] : 999999;
}

function getProficiency(level) {
    if (level <= 4) return 2;
    if (level <= 8) return 3;
    return 4;
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}

function escapeAttr(str) {
    return (str || '').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}
