// Adventure screen renderer and info panel

export function renderAdventure(container, state, handlers) {
    if (!state) {
        container.innerHTML = '<div class="loading">Loading adventure</div>';
        return;
    }

    container.innerHTML = `
    <div class="adventure-layout">
        <div class="story-panel">
            <div class="story-header">
                <button class="btn-back" id="advBackBtn" style="position:static;">&larr;</button>
                <h2>${escapeHtml(state.name || 'Adventure')}</h2>
                <span class="scene-location" id="sceneLocation">${escapeHtml(state.current_scene?.location || 'Unknown')}</span>
                <span class="cost-display" id="costDisplay" title="Session cost">$0.0000</span>
                <button class="btn-options" id="optionsBtn" title="Options">&#9881;</button>
            </div>
            <div class="story-content" id="storyContent">
                <div class="loading-narrative">
                    <div class="d20-spinner"></div>
                    <span class="loading-text">Your adventure is loading</span>
                </div>
            </div>
        </div>
        <div class="info-panel">
            <div class="info-tabs">
                <button class="info-tab active" data-tab="stats">Status</button>
                <button class="info-tab" data-tab="inventory">Items</button>
                <button class="info-tab" data-tab="map">Map</button>
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

    // Options button
    document.getElementById('optionsBtn')?.addEventListener('click', () => {
        const event = new Event('show-options');
        document.dispatchEvent(event);
    });

    // State update listener
    const stateHandler = (e) => {
        Object.assign(state, e.detail);
        renderTab(activeTab, state);
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
            renderStatus(el, state);
            break;
        case 'inventory':
            renderInventory(el, state);
            break;
        case 'map':
            renderMap(el, state);
            break;
        case 'quests':
            renderQuests(el, state);
            break;
    }
}

function renderStatus(el, state) {
    const c = state.character || {};
    const s = c.stats || {};
    const hpPct = c.max_hp > 0 ? (Math.max(c.hp, 0) / c.max_hp * 100) : 0;
    const hpClass = hpPct > 50 ? 'hp' : hpPct > 25 ? 'hp warning' : 'hp critical';
    const xpNext = getXpNext(c.level || 1);
    const xpPct = xpNext > 0 ? ((c.xp || 0) / xpNext * 100) : 0;

    let html = `
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

        ${renderConditions(c.conditions)}
    `;

    // Abilities section (merged from Skills tab)
    const abilities = state.abilities || [];
    const slots = state.spell_slots || {};

    html += '<div class="abilities-section">';
    html += '<div class="section-title">Abilities & Skills</div>';

    if (abilities.length === 0) {
        html += '<div class="empty-state" style="padding:8px 0;">No abilities yet.</div>';
    } else {
        abilities.forEach(a => {
            html += `<div class="ability-entry">
                <div style="color:var(--text-gold); font-family:var(--font-medieval);">${escapeHtml(a.name)}</div>
                <div style="font-size:12px; color:var(--text-muted); margin-top:2px;">${escapeHtml(a.description)}</div>
                ${a.uses_per_rest != null ? `<div style="font-size:11px; color:var(--text-light); margin-top:2px;">Uses: ${a.uses_remaining ?? '?'}/${a.uses_per_rest}</div>` : ''}
            </div>`;
        });
    }

    if (slots.level_1 > 0 || slots.level_2 > 0 || slots.level_3 > 0) {
        html += '<div style="margin-top:12px; font-size:12px; color:var(--text-gold);">Spell Slots</div>';
        if (slots.level_1 > 0) html += renderSlotRow('1st', slots.level_1, slots.level_1_used || 0);
        if (slots.level_2 > 0) html += renderSlotRow('2nd', slots.level_2, slots.level_2_used || 0);
        if (slots.level_3 > 0) html += renderSlotRow('3rd', slots.level_3, slots.level_3_used || 0);
    }

    html += '</div>';

    el.innerHTML = html;
}

const CONDITION_DESCRIPTIONS = {
    'poisoned': 'Disadvantage on attacks & ability checks. 1d4 poison damage/turn.',
    'burning': '1d6 fire damage at the start of each turn until extinguished.',
    'on fire': '1d6 fire damage at the start of each turn until extinguished.',
    'bleeding': '1d4 damage at the start of each turn until healed.',
    'blinded': "Can't see. Disadvantage on attack rolls.",
    'frightened': 'Disadvantage on ability checks and attacks while source is visible.',
    'stunned': "Can't move or act. Fails STR/DEX saves.",
    'paralyzed': "Can't move or act. Auto-fail STR/DEX saves. Melee hits are crits.",
    'exhaustion': 'Disadvantage on ability checks. Speed halved.',
};

function renderConditions(conditions) {
    if (!conditions || conditions.length === 0) return '';

    let html = '<div class="conditions-section"><div class="section-title">Status Effects</div>';
    conditions.forEach(c => {
        const desc = CONDITION_DESCRIPTIONS[c.toLowerCase()] || 'Active status effect.';
        html += `<div class="condition-entry">
            <div class="condition-name">${escapeHtml(c)}</div>
            <div class="condition-desc">${escapeHtml(desc)}</div>
        </div>`;
    });
    html += '</div>';
    return html;
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

function renderSlotRow(label, total, used) {
    let dots = '';
    for (let i = 0; i < total; i++) {
        const filled = i < (total - used);
        dots += `<span style="display:inline-block;width:14px;height:14px;border-radius:50%;border:1px solid var(--border-gold);background:${filled ? 'var(--text-gold)' : 'transparent'};margin-right:3px;"></span>`;
    }
    return `<div style="padding:4px 0;font-size:12px;color:var(--text-muted);">${label}: ${dots}</div>`;
}

function renderInventory(el, state) {
    const eq = state.equipment || {};
    const inv = state.inventory || { items: [], gold: 0 };
    const typeIcons = { weapon: '\u2694', armor: '\u{1F6E1}', potion: '\u{1F9EA}', scroll: '\u{1F4DC}', misc: '\u{1F4E6}' };

    const SLOT_LABELS = [
        ['head', 'Head'], ['amulet', 'Amulet'], ['main_hand', 'Main Hand'],
        ['off_hand', 'Off Hand'], ['chest', 'Chest'], ['hands', 'Hands'],
        ['ring1', 'Ring 1'], ['ring2', 'Ring 2'], ['legs', 'Legs'],
        ['feet', 'Feet'], ['back', 'Back'],
    ];

    let html = '<div class="section-title">Equipped</div>';
    html += '<div class="equip-slots">';
    SLOT_LABELS.forEach(([key, label]) => {
        const item = eq[key];
        if (item) {
            const name = item.enchantment ? `${item.enchantment.name_prefix} ${item.name}` : item.name;
            const rarityClass = (item.rarity || 'common').toLowerCase();
            html += `<div class="equip-slot filled" title="${escapeAttr(item.description || '')}">
                <span class="slot-label">${label}</span>
                <span class="slot-item rarity-${rarityClass}">${escapeHtml(name)}</span>
            </div>`;
        } else {
            html += `<div class="equip-slot empty">
                <span class="slot-label">${label}</span>
                <span class="slot-item empty-slot">Empty</span>
            </div>`;
        }
    });
    html += '</div>';

    // Gold
    const gold = state.character?.gold || inv.gold || 0;
    html += `<div class="gold-display">\u{1FA99} ${gold} Gold</div>`;

    // Backpack
    html += '<div class="section-title" style="margin-top:12px;">Backpack</div>';
    if (!inv.items || inv.items.length === 0) {
        html += '<div class="empty-state" style="padding:8px;">Empty</div>';
    } else {
        html += '<ul class="item-list">';
        inv.items.forEach(item => {
            const icon = typeIcons[item.item_type] || '\u{1F4E6}';
            const qty = item.quantity > 1 ? ` (x${item.quantity})` : '';
            const name = item.enchantment ? `${item.enchantment.name_prefix} ${item.name}` : item.name;
            const rarityClass = (item.rarity || 'common').toLowerCase();
            html += `<li class="item-entry" title="${escapeAttr(item.description || '')}">
                <span class="item-icon">${icon}</span>
                <span class="item-name rarity-${rarityClass}">${escapeHtml(name)}${qty}</span>
                <span class="item-type">${item.item_type || 'misc'}</span>
            </li>`;
        });
        html += '</ul>';
    }

    el.innerHTML = html;
}

function renderMap(el, state) {
    // Show world map if available and not in a dungeon
    const world = state.world;
    const dungeon = state.dungeon;

    if (world && !dungeon) {
        renderWorldMap(el, world);
        return;
    }

    if (!dungeon) {
        el.innerHTML = '<div class="empty-state">No map available.</div>';
        return;
    }

    const floor = dungeon.floors[dungeon.current_floor];
    if (!floor) {
        el.innerHTML = '<div class="empty-state">Invalid floor.</div>';
        return;
    }

    // Floor selector
    let html = `<div class="map-header">
        <div class="dungeon-name">${escapeHtml(dungeon.name)}</div>
        <div class="floor-selector">`;
    dungeon.floors.forEach((f, i) => {
        const active = i === dungeon.current_floor ? ' active' : '';
        html += `<span class="floor-btn${active}">F${f.level}</span>`;
    });
    html += '</div></div>';

    // Current room info
    const currentRoom = floor.rooms[dungeon.current_room];
    if (currentRoom) {
        const typeLabel = currentRoom.room_type || 'unknown';
        const clearedBadge = currentRoom.cleared ? ' <span class="room-cleared-badge">Cleared</span>' : '';
        html += `<div class="current-room-info">
            <span class="room-label">${escapeHtml(currentRoom.name)}</span>
            <span class="room-type-badge">${typeLabel}</span>${clearedBadge}
        </div>`;
    }

    // Render grid map
    const W = floor.width || 40;
    const H = floor.height || 30;
    const CELL = 7;

    // Build a 2D grid of cell types
    const grid = Array.from({length: H}, () => Array(W).fill('fog'));

    // Mark rooms
    floor.rooms.forEach((room, idx) => {
        if (!room.discovered) return;
        for (let ry = room.y; ry < room.y + room.h && ry < H; ry++) {
            for (let rx = room.x; rx < room.x + room.w && rx < W; rx++) {
                let cellClass = 'room';
                if (idx === dungeon.current_room) cellClass = 'current';
                else if (room.room_type === 'Boss') cellClass = 'boss';
                else if (room.room_type === 'Combat' && !room.cleared) cellClass = 'combat';
                else if (room.room_type === 'Stairs') cellClass = 'stairs';
                else if (room.cleared) cellClass = 'cleared';
                else if (room.room_type === 'Treasure') cellClass = 'treasure';
                grid[ry][rx] = cellClass;
            }
        }
    });

    // Mark corridors
    floor.corridors.forEach(cor => {
        if (!cor.discovered) return;
        cor.cells.forEach(([cx, cy]) => {
            if (cy < H && cx < W && grid[cy][cx] === 'fog') {
                grid[cy][cx] = 'corridor';
            }
        });
    });

    // Render as CSS grid
    html += `<div class="dungeon-grid" style="grid-template-columns:repeat(${W},${CELL}px);grid-template-rows:repeat(${H},${CELL}px);">`;
    for (let y = 0; y < H; y++) {
        for (let x = 0; x < W; x++) {
            html += `<div class="map-cell ${grid[y][x]}"></div>`;
        }
    }
    html += '</div>';

    // Legend
    html += `<div class="map-legend">
        <span><span class="legend-swatch current"></span>You</span>
        <span><span class="legend-swatch room"></span>Room</span>
        <span><span class="legend-swatch cleared"></span>Cleared</span>
        <span><span class="legend-swatch combat"></span>Enemies</span>
        <span><span class="legend-swatch stairs"></span>Stairs</span>
        <span><span class="legend-swatch treasure"></span>Treasure</span>
    </div>`;

    el.innerHTML = html;
}

function renderWorldMap(el, world) {
    const currentLoc = world.locations[world.current_location];

    let html = `<div class="map-header">
        <div class="dungeon-name">${escapeHtml(world.name)}</div>
    </div>`;

    // Current location
    html += `<div class="current-room-info">
        <span class="room-label">${escapeHtml(currentLoc.name)}</span>
        <span class="room-type-badge">${currentLoc.location_type}</span>
    </div>`;

    // World map as positioned nodes
    html += '<div class="world-map-container">';

    // Draw connections first (as lines)
    html += '<svg class="world-map-svg" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet">';
    world.connections.forEach(conn => {
        if (!conn.discovered) return;
        const from = world.locations[conn.from];
        const to = world.locations[conn.to];
        const dangerColor = conn.danger_level === 0 ? '#2a5a2a' : conn.danger_level === 1 ? '#5a5a2a' : conn.danger_level === 2 ? '#5a3a1a' : '#5a1a1a';
        html += `<line x1="${from.x*100}" y1="${from.y*100}" x2="${to.x*100}" y2="${to.y*100}" stroke="${dangerColor}" stroke-width="0.4" opacity="0.6"/>`;
    });
    html += '</svg>';

    // Draw location nodes
    world.locations.forEach((loc, i) => {
        if (!loc.discovered) return;
        const isCurrent = i === world.current_location;
        const typeClass = loc.location_type.toLowerCase ? loc.location_type.toLowerCase() : loc.location_type;
        const icon = {town:'\u{1F3E0}', dungeon:'\u{1F480}', wilderness:'\u{1F332}', landmark:'\u2728', camp:'\u{26FA}', tower:'\u{1F3F0}'}[typeClass] || '\u25CF';
        html += `<div class="world-node ${typeClass}${isCurrent ? ' current' : ''}" style="left:${loc.x*100}%;top:${loc.y*100}%" title="${escapeAttr(loc.description)}">
            <span class="node-icon">${icon}</span>
            <span class="node-label">${escapeHtml(loc.name)}</span>
        </div>`;
    });

    html += '</div>';

    // Legend
    html += `<div class="map-legend" style="margin-top:4px;">
        <span>\u{1F3E0} Town</span>
        <span>\u{1F480} Dungeon</span>
        <span>\u{1F332} Wild</span>
        <span>\u{1F3F0} Tower</span>
        <span>\u26FA Camp</span>
    </div>`;

    el.innerHTML = html;
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
