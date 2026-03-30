// Adventure screen renderer and info panel


function equipWsCall(action, data) {
    if (window.rqWs && window.rqWs.send) {
        window.rqWs.send({ type: action, ...data });
    }
}

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
    const inCombat = state.combat?.active === true;

    const SLOT_LABELS = [
        ['head', 'Head'], ['amulet', 'Amulet'], ['main_hand', 'Main Hand'],
        ['off_hand', 'Off Hand'], ['chest', 'Chest'], ['hands', 'Hands'],
        ['ring1', 'Ring 1'], ['ring2', 'Ring 2'], ['legs', 'Legs'],
        ['feet', 'Feet'], ['back', 'Back'],
    ];

    let html = '<div class="section-title">Equipped</div>';
    if (!inCombat) html += '<div class="equip-hint">Click to unequip</div>';
    html += '<div class="equip-slots">';
    SLOT_LABELS.forEach(([key, label]) => {
        const item = eq[key];
        if (item) {
            const name = item.enchantment ? `${item.enchantment.name_prefix} ${item.name}` : item.name;
            const rarityClass = (item.rarity || 'common').toLowerCase();
            const clickable = !inCombat ? ' clickable' : '';
            html += `<div class="equip-slot filled${clickable}" data-slot="${key}" title="${escapeAttr(item.description || '')}${!inCombat ? ' — click to unequip' : ''}">
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
    if (!inCombat) html += '<div class="equip-hint">Click equippable items to equip</div>';
    if (!inv.items || inv.items.length === 0) {
        html += '<div class="empty-state" style="padding:8px;">Empty</div>';
    } else {
        html += '<ul class="item-list">';
        inv.items.forEach(item => {
            const icon = typeIcons[item.item_type] || '\u{1F4E6}';
            const qty = item.quantity > 1 ? ` (x${item.quantity})` : '';
            const name = item.enchantment ? `${item.enchantment.name_prefix} ${item.name}` : item.name;
            const rarityClass = (item.rarity || 'common').toLowerCase();
            const equippable = !inCombat && item.slot;
            const clickable = equippable ? ' clickable' : '';
            const itemName = item.enchantment ? `${item.enchantment.name_prefix} ${item.name}` : item.name;
            html += `<li class="item-entry${clickable}" ${equippable ? `data-equip-name="${escapeAttr(itemName)}"` : ''} title="${escapeAttr(item.description || '')}${equippable ? ' — click to equip' : ''}">
                <span class="item-icon">${icon}</span>
                <span class="item-name rarity-${rarityClass}">${escapeHtml(name)}${qty}</span>
                <span class="item-type">${item.item_type || 'misc'}</span>
            </li>`;
        });
        html += '</ul>';
    }

    el.innerHTML = html;

    // Bind click handlers for equip/unequip (only outside combat)
    if (!inCombat && state.id) {
        el.querySelectorAll('.equip-slot.clickable').forEach(slot => {
            slot.addEventListener('click', async () => {
                slot.classList.add('loading');
                await equipApiCall(state.id, 'unequip', { slot: slot.dataset.slot });
            });
        });
        el.querySelectorAll('.item-entry.clickable').forEach(entry => {
            entry.addEventListener('click', async () => {
                entry.classList.add('loading');
                await equipApiCall(state.id, 'equip', { item_name: entry.dataset.equipName });
            });
        });
    }
}

function renderMap(el, state) {
    const dungeon = state.dungeon;
    const mapView = state.map_view;

    if (mapView && !dungeon) {
        renderHexMap(el, mapView);
        return;
    }

    if (dungeon) {
        renderDungeonMap(el, dungeon);
        return;
    }

    el.innerHTML = '<div class="empty-state">No map available.</div>';
}

function renderHexMap(el, mapView) {
    const current = mapView.current;
    let html = '<div class="map-header">';
    if (current) {
        html += '<div class="dungeon-name">' + escapeHtml(current.name) + '</div>';
        html += '<div style="font-size:11px;color:var(--text-muted);">' + escapeHtml(current.region || '') + ' &middot; ' + escapeHtml(current.biome || '') + '</div>';
    }
    html += '</div>';

    const hexes = mapView.hexes || [];
    if (hexes.length > 0) {
        const HEX_SIZE = 28;
        const HEX_W = HEX_SIZE * Math.sqrt(3);
        const HEX_H = HEX_SIZE * 2;

        const hexData = hexes.map(function(h) {
            var px = HEX_SIZE * Math.sqrt(3) * (h.q + h.r / 2.0);
            var py = HEX_SIZE * 1.5 * h.r;
            return { hex: h, px: px, py: py };
        });

        var minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
        hexData.forEach(function(d) {
            if (d.px - HEX_W/2 < minX) minX = d.px - HEX_W/2;
            if (d.px + HEX_W/2 > maxX) maxX = d.px + HEX_W/2;
            if (d.py - HEX_H/2 < minY) minY = d.py - HEX_H/2;
            if (d.py + HEX_H/2 > maxY) maxY = d.py + HEX_H/2;
        });

        var svgW = maxX - minX + 10;
        var svgH = maxY - minY + 10;
        var offX = -minX + 5;
        var offY = -minY + 5;

        html += '<svg class="hex-map-svg" viewBox="0 0 ' + svgW + ' ' + svgH + '" preserveAspectRatio="xMidYMid meet" style="width:100%;max-height:320px;">';

        hexData.forEach(function(d) {
            var cx = d.px + offX;
            var cy = d.py + offY;
            var h = d.hex;

            var points = [];
            for (var i = 0; i < 6; i++) {
                var angle = Math.PI / 180 * (60 * i - 30);
                points.push((cx + HEX_SIZE * Math.cos(angle)).toFixed(1) + ',' + (cy + HEX_SIZE * Math.sin(angle)).toFixed(1));
            }
            var pointsStr = points.join(' ');

            var fillColor = '#1a1a2e';
            if (!h.discovered) {
                fillColor = '#0d0d15';
            } else {
                var biome = (h.biome || '').toLowerCase();
                if (biome === 'forest') fillColor = '#1a3a1a';
                else if (biome === 'plains' || biome === 'grassland') fillColor = '#2a3a1a';
                else if (biome === 'mountain' || biome === 'mountains') fillColor = '#2a2a3a';
                else if (biome === 'desert') fillColor = '#3a3a1a';
                else if (biome === 'swamp') fillColor = '#1a2a2a';
                else if (biome === 'tundra' || biome === 'snow') fillColor = '#2a3a4a';
                else if (biome === 'coast' || biome === 'ocean') fillColor = '#1a2a4a';
                else if (biome === 'jungle') fillColor = '#0a3a0a';
                else if (biome === 'volcanic') fillColor = '#3a1a0a';
                else fillColor = '#1a2a1a';
            }

            var strokeColor = h.current ? '#d4a843' : '#333';
            var strokeWidth = h.current ? 2.5 : 0.8;

            html += '<polygon points="' + pointsStr + '" fill="' + fillColor + '" stroke="' + strokeColor + '" stroke-width="' + strokeWidth + '"/>';

            if (h.discovered) {
                var icons = [];
                if (h.current) icons.push('\u25c9');
                if (h.has_town) icons.push('\ud83c\udfe0');
                if (h.has_dungeon) icons.push('\ud83d\udc80');
                if (h.has_tower) icons.push('\ud83c\udff0');
                if (h.has_exchange) icons.push('\ud83e\ude99');

                if (icons.length > 0) {
                    var fsize = icons.length > 2 ? 8 : 10;
                    var fcolor = h.current ? '#d4a843' : '#ccc';
                    html += '<text x="' + cx + '" y="' + (cy + 1) + '" text-anchor="middle" dominant-baseline="central" font-size="' + fsize + '" fill="' + fcolor + '">' + icons.join('') + '</text>';
                }
            }
        });

        html += '</svg>';
    }

    var directions = mapView.directions || [];
    if (directions.length > 0) {
        html += '<div class="hex-directions">';
        directions.forEach(function(dir) {
            var biomeStr = dir.biome && dir.biome !== '?' ? ' (' + dir.biome + ')' : '';
            html += '<div class="hex-dir-entry">';
            html += '<span class="hex-dir-name">' + escapeHtml(dir.direction) + '</span> ';
            html += '<span class="hex-dir-target">' + escapeHtml(dir.name) + biomeStr + '</span>';
            html += '</div>';
        });
        html += '</div>';
    }

    if (current) {
        html += '<div class="hex-features">';
        if (current.has_town) html += '<span class="hex-feat">\ud83c\udfe0 Town</span>';
        if (current.has_dungeon) html += '<span class="hex-feat">\ud83d\udc80 Dungeon</span>';
        if (current.has_tower) html += '<span class="hex-feat">\ud83c\udff0 ' + escapeHtml(current.tower_name || 'Tower') + '</span>';
        if (current.has_exchange) html += '<span class="hex-feat">\ud83e\ude99 Exchange</span>';
        if (current.has_guild_hall) html += '<span class="hex-feat">\u2694 Guild Hall</span>';
        html += '</div>';
    }

    html += '<div class="map-legend" style="margin-top:4px;">';
    html += '<span>\u25c9 You</span>';
    html += '<span>\ud83c\udfe0 Town</span>';
    html += '<span>\ud83d\udc80 Dungeon</span>';
    html += '<span>\ud83c\udff0 Tower</span>';
    html += '</div>';

    el.innerHTML = html;
}

function renderDungeonMap(el, dungeon) {
    const floor = dungeon.floors[dungeon.current_floor];
    if (!floor) {
        el.innerHTML = '<div class="empty-state">Invalid floor.</div>';
        return;
    }

    let html = '<div class="map-header"><div class="dungeon-name">' + escapeHtml(dungeon.name) + '</div><div class="floor-selector">';
    dungeon.floors.forEach(function(f, i) {
        var active = i === dungeon.current_floor ? ' active' : '';
        html += '<span class="floor-btn' + active + '">F' + f.level + '</span>';
    });
    html += '</div></div>';

    var currentRoom = floor.rooms[dungeon.current_room];
    if (currentRoom) {
        var typeLabel = currentRoom.room_type || 'unknown';
        var clearedBadge = currentRoom.cleared ? ' <span class="room-cleared-badge">Cleared</span>' : '';
        html += '<div class="current-room-info"><span class="room-label">' + escapeHtml(currentRoom.name) + '</span><span class="room-type-badge">' + typeLabel + '</span>' + clearedBadge + '</div>';
    }

    var W = floor.width || 40;
    var H = floor.height || 30;
    var CELL = 7;
    var grid = Array.from({length: H}, function() { return Array(W).fill('fog'); });

    floor.rooms.forEach(function(room, idx) {
        if (!room.discovered) return;
        for (var ry = room.y; ry < room.y + room.h && ry < H; ry++) {
            for (var rx = room.x; rx < room.x + room.w && rx < W; rx++) {
                var cellClass = 'room';
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

    floor.corridors.forEach(function(cor) {
        if (!cor.discovered) return;
        cor.cells.forEach(function(cell) {
            var cx = cell[0], cy = cell[1];
            if (cy < H && cx < W && grid[cy][cx] === 'fog') {
                grid[cy][cx] = 'corridor';
            }
        });
    });

    html += '<div class="dungeon-grid" style="grid-template-columns:repeat(' + W + ',' + CELL + 'px);grid-template-rows:repeat(' + H + ',' + CELL + 'px);">';
    for (var y = 0; y < H; y++) {
        for (var x = 0; x < W; x++) {
            html += '<div class="map-cell ' + grid[y][x] + '"></div>';
        }
    }
    html += '</div>';

    html += '<div class="map-legend">';
    html += '<span><span class="legend-swatch current"></span>You</span>';
    html += '<span><span class="legend-swatch room"></span>Room</span>';
    html += '<span><span class="legend-swatch cleared"></span>Cleared</span>';
    html += '<span><span class="legend-swatch combat"></span>Enemies</span>';
    html += '<span><span class="legend-swatch stairs"></span>Stairs</span>';
    html += '<span><span class="legend-swatch treasure"></span>Treasure</span>';
    html += '</div>';

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
