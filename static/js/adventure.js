// Adventure screen renderer and info panel

// ===== ITEM CONTEXT MENU =====
let activeContextMenu = null;

function dismissContextMenu() {
    if (activeContextMenu) {
        activeContextMenu.remove();
        activeContextMenu = null;
    }
}

document.addEventListener('click', (e) => {
    if (activeContextMenu && !activeContextMenu.contains(e.target)) {
        dismissContextMenu();
    }
});
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') dismissContextMenu();
});

function showItemContextMenu(event, item, options) {
    event.preventDefault();
    event.stopPropagation();
    dismissContextMenu();

    const menu = document.createElement('div');
    menu.className = 'item-context-menu';

    const header = document.createElement('div');
    header.className = 'ctx-menu-header';
    const rarityClass = (item.rarity || 'common').toLowerCase();
    header.className = 'ctx-menu-header rarity-' + rarityClass;
    const displayName = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
    header.textContent = displayName;
    menu.appendChild(header);

    const actions = [];

    if (options.canEquip) {
        actions.push({ label: 'Equip', icon: '\u2694', action: () => {
            if (window.rqWs) window.rqWs.send({ type: 'equip_item', item_name: options.equipName });
        }});
    }
    if (options.canUnequip) {
        actions.push({ label: 'Unequip', icon: '\u{1F44B}', action: () => {
            if (window.rqWs) window.rqWs.send({ type: 'unequip_item', slot: options.slot });
        }});
    }
    actions.push({ label: 'Info', icon: '\u{1F4CB}', action: () => showItemInfoModal(item) });
    actions.push({ label: 'Examine', icon: '\u{1F50D}', action: () => examineItem(item) });
    if (options.inBackpack) {
        actions.push({ label: 'Drop', icon: '\u274C', action: () => {
            if (window.rqWs) window.rqWs.send({ type: 'drop_item', item_name: displayName });
        }});
    }

    actions.forEach(a => {
        const btn = document.createElement('div');
        btn.className = 'ctx-menu-item';
        btn.innerHTML = '<span class="ctx-icon">' + a.icon + '</span> ' + escapeHtml(a.label);
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            dismissContextMenu();
            a.action();
        });
        menu.appendChild(btn);
    });

    document.body.appendChild(menu);
    const rect = menu.getBoundingClientRect();
    let x = event.clientX || 0;
    let y = event.clientY || 0;
    if (x + rect.width > window.innerWidth) x = window.innerWidth - rect.width - 8;
    if (y + rect.height > window.innerHeight) y = window.innerHeight - rect.height - 8;
    if (x < 0) x = 4;
    if (y < 0) y = 4;
    menu.style.left = x + 'px';
    menu.style.top = y + 'px';

    activeContextMenu = menu;
}

function showItemInfoModal(item) {
    const existing = document.querySelector('.item-info-modal');
    if (existing) existing.remove();

    const modal = document.createElement('div');
    modal.className = 'item-info-modal';

    const displayName = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
    const rarityClass = (item.rarity || 'common').toLowerCase();

    let html = '<div class="item-info-content">';
    html += '<div class="item-info-name rarity-' + rarityClass + '">' + escapeHtml(displayName) + '</div>';
    if (item.rarity && item.rarity.toLowerCase() !== 'common') {
        html += '<div class="item-info-rarity rarity-' + rarityClass + '">' + escapeHtml(item.rarity) + '</div>';
    }
    html += '<div class="item-info-divider"></div>';

    if (item.description) {
        html += '<div class="item-info-desc">' + escapeHtml(item.description) + '</div>';
    }

    html += '<div class="item-info-stats">';
    if (item.slot) html += '<div class="item-info-stat"><span class="iis-label">Slot</span><span class="iis-val">' + escapeHtml(formatSlotLabel(item.slot)) + '</span></div>';
    if (item.item_type) html += '<div class="item-info-stat"><span class="iis-label">Type</span><span class="iis-val">' + escapeHtml(item.item_type) + '</span></div>';
    if (item.damage_dice) html += '<div class="item-info-stat"><span class="iis-label">Damage</span><span class="iis-val damage">' + escapeHtml(item.damage_dice) + '</span></div>';
    if (item.ac_bonus) html += '<div class="item-info-stat"><span class="iis-label">AC Bonus</span><span class="iis-val ac">+' + item.ac_bonus + '</span></div>';
    if (item.tier != null) html += '<div class="item-info-stat"><span class="iis-label">Tier</span><span class="iis-val">' + item.tier + '</span></div>';
    if (item.value != null) html += '<div class="item-info-stat"><span class="iis-label">Value</span><span class="iis-val gold">' + item.value + ' gold</span></div>';
    if (item.enchantment) {
        html += '<div class="item-info-stat"><span class="iis-label">Enchantment</span><span class="iis-val enchant">' + escapeHtml(item.enchantment.name_prefix) + '</span></div>';
        if (item.enchantment.description) {
            html += '<div class="item-info-enchant-desc">' + escapeHtml(item.enchantment.description) + '</div>';
        }
    }
    if (item.properties && item.properties.length > 0) {
        html += '<div class="item-info-props-title">Properties</div>';
        item.properties.forEach(function(p) {
            html += '<div class="item-info-prop">' + escapeHtml(typeof p === 'string' ? p : p.name || JSON.stringify(p)) + '</div>';
        });
    }
    html += '</div>';

    html += '<button class="stone-btn item-info-close">Close</button>';
    html += '</div>';
    modal.innerHTML = html;

    document.body.appendChild(modal);
    modal.querySelector('.item-info-close').addEventListener('click', () => modal.remove());
    modal.addEventListener('click', (e) => { if (e.target === modal) modal.remove(); });
    document.addEventListener('keydown', function closeOnEsc(e) {
        if (e.key === 'Escape') { modal.remove(); document.removeEventListener('keydown', closeOnEsc); }
    });
}

function examineItem(item) {
    const storyContent = document.querySelector('.story-content');
    if (!storyContent) return;
    const displayName = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
    const desc = item.description || 'You see nothing special about it.';
    const div = document.createElement('div');
    div.className = 'narrative-block examine';
    div.innerHTML = '<span style="color:var(--text-gold);">You examine the ' + escapeHtml(displayName) + ':</span> ' + escapeHtml(desc);
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

function bindLongPress(element, callback) {
    let timer = null;
    let moved = false;
    element.addEventListener('touchstart', function(e) {
        moved = false;
        timer = setTimeout(function() {
            if (!moved) {
                e.preventDefault();
                const touch = e.touches[0];
                callback({ preventDefault: function(){}, stopPropagation: function(){}, clientX: touch.clientX, clientY: touch.clientY });
            }
        }, 500);
    }, { passive: false });
    element.addEventListener('touchmove', function() { moved = true; if (timer) clearTimeout(timer); });
    element.addEventListener('touchend', function() { if (timer) clearTimeout(timer); });
    element.addEventListener('touchcancel', function() { if (timer) clearTimeout(timer); });
}

const SLOT_LABEL_MAP = {
    head: 'Head', chest: 'Chest', legs: 'Legs', feet: 'Feet', hands: 'Hands',
    main_hand: 'Main Hand', off_hand: 'Off Hand', ring1: 'Ring 1', ring2: 'Ring 2',
    ring: 'Ring', amulet: 'Amulet', neck: 'Neck', back: 'Back',
};

function formatSlotLabel(slot) {
    return SLOT_LABEL_MAP[slot] || slot.replace(/_/g, ' ').replace(/\b\w/g, function(c) { return c.toUpperCase(); });
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
                <button class="info-tab" data-tab="skills">Skills</button>
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
        case 'skills':
            renderSkills(el, state);
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
        ${renderEquipOverview(state)}
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

function renderEquipOverview(state) {
    const eq = state.equipment || {};
    const equipped = Object.entries(eq).filter(function(e) { return e[1] != null; });
    if (equipped.length === 0) return '';

    let totalAc = 10;
    let weaponDamage = null;
    let weaponName = null;
    let bonuses = [];

    equipped.forEach(function(entry) {
        var slot = entry[0], item = entry[1];
        if (item.ac_bonus) totalAc += item.ac_bonus;
        if (slot === 'main_hand' && item.damage_dice) {
            weaponDamage = item.damage_dice;
            weaponName = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
        }
        if (item.enchantment && item.enchantment.description) {
            bonuses.push(item.enchantment.name_prefix + ': ' + item.enchantment.description);
        }
    });

    let html = '<div class="equip-overview-section">';
    html += '<div class="section-title">Equipment Summary</div>';
    html += '<div class="equip-overview-stats">';
    html += '<div class="eo-stat"><span class="eo-stat-label">Total AC</span><span class="eo-stat-value">' + totalAc + '</span></div>';
    if (weaponDamage) {
        html += '<div class="eo-stat"><span class="eo-stat-label">Weapon</span><span class="eo-stat-value">' + escapeHtml(weaponDamage) + '</span></div>';
    }
    html += '</div>';

    if (bonuses.length > 0) {
        html += '<div class="eo-bonuses">';
        bonuses.forEach(function(b) {
            html += '<div class="eo-bonus">' + escapeHtml(b) + '</div>';
        });
        html += '</div>';
    }

    html += '<div class="eo-items">';
    equipped.forEach(function(entry) {
        var slot = entry[0], item = entry[1];
        var name = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
        var rarityClass = (item.rarity || 'common').toLowerCase();
        var stat = item.damage_dice ? item.damage_dice : item.ac_bonus ? 'AC +' + item.ac_bonus : '';
        html += '<div class="eo-item">';
        html += '<span class="eo-item-slot">' + escapeHtml(formatSlotLabel(slot)) + '</span>';
        html += '<span class="eo-item-name rarity-' + rarityClass + '">' + escapeHtml(name) + '</span>';
        if (stat) html += '<span class="eo-item-stat">' + escapeHtml(stat) + '</span>';
        html += '</div>';
    });
    html += '</div></div>';
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
    if (!inCombat) html += '<div class="equip-hint">Click to unequip \u00b7 Right-click for options</div>';
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
    if (!inCombat) html += '<div class="equip-hint">Click to equip \u00b7 Right-click for options</div>';
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
            slot.addEventListener('click', () => {
                slot.classList.add('loading');
                if (window.rqWs) window.rqWs.send({ type: 'unequip_item', slot: slot.dataset.slot });
            });
        });
        el.querySelectorAll('.item-entry.clickable').forEach(entry => {
            entry.addEventListener('click', () => {
                entry.classList.add('loading');
                if (window.rqWs) window.rqWs.send({ type: 'equip_item', item_name: entry.dataset.equipName });
            });
        });
    }

    // Context menu handlers (right-click / long-press) for equipped items
    el.querySelectorAll('.equip-slot.filled').forEach(function(slot) {
        var slotKey = slot.dataset.slot;
        var item = eq[slotKey];
        if (!item) return;
        var handler = function(e) {
            showItemContextMenu(e, item, {
                canUnequip: !inCombat,
                slot: slotKey,
                inBackpack: false,
            });
        };
        slot.addEventListener('contextmenu', handler);
        bindLongPress(slot, handler);
    });

    // Context menu handlers for backpack items
    var itemEntries = el.querySelectorAll('.item-entry');
    itemEntries.forEach(function(entry, idx) {
        var item = inv.items[idx];
        if (!item) return;
        var itemName = item.enchantment ? item.enchantment.name_prefix + ' ' + item.name : item.name;
        var handler = function(e) {
            showItemContextMenu(e, item, {
                canEquip: !inCombat && !!item.slot,
                equipName: itemName,
                inBackpack: true,
            });
        };
        entry.addEventListener('contextmenu', handler);
        bindLongPress(entry, handler);
    });
}

function renderSkills(el, state) {
    // Request skills from server if we don't have them cached
    if (!state._skills_cache) {
        if (window.rqWs) {
            window.rqWs.send({ type: 'get_skills' });
        }
        el.innerHTML = '<div class="loading">Loading skills</div>';

        // Listen for skills response
        var handler = function(e) {
            if (e.detail && e.detail.skills) {
                state._skills_cache = e.detail.skills;
                document.removeEventListener('skill-list-received', handler);
                renderSkillsList(el, state._skills_cache);
            }
        };
        document.addEventListener('skill-list-received', handler);
        return;
    }
    renderSkillsList(el, state._skills_cache);
}

function renderSkillsList(el, skills) {
    if (!skills || skills.length === 0) {
        el.innerHTML = '<div class="empty-state">No skills data available.</div>';
        return;
    }

    var combatKeys = ['swordsmanship','archery','defense','tactics','dual_wielding','heavy_weapons','polearms','unarmed','shield_use','thrown_weapons','melee','ranged','combat'];
    var craftKeys = ['mining','smithing','woodcutting','carpentry','herbalism','alchemy','cooking','leatherworking','tailoring','jewelcrafting','enchanting','fishing','farming','brewing','fletching','masonry','tinkering','runecrafting','survival','gathering','smelting','weaving','pottery'];
    var agilityKeys = ['stealth','lockpicking','pickpocket','acrobatics','athletics','perception','tracking','climbing','swimming','evasion','thievery'];
    var knowledgeKeys = ['persuasion','intimidation','deception','bartering','leadership','diplomacy','performance','lore','medicine','arcana','nature','religion','history','investigation','bard','music','haggling'];

    var categories = {};
    skills.forEach(function(s) {
        var cat = 'General';
        if (combatKeys.some(function(k) { return s.id.indexOf(k) !== -1; })) cat = 'Combat';
        else if (craftKeys.some(function(k) { return s.id.indexOf(k) !== -1; })) cat = 'Crafting & Gathering';
        else if (agilityKeys.some(function(k) { return s.id.indexOf(k) !== -1; })) cat = 'Agility & Stealth';
        else if (knowledgeKeys.some(function(k) { return s.id.indexOf(k) !== -1; })) cat = 'Knowledge & Social';
        if (!categories[cat]) categories[cat] = [];
        categories[cat].push(s);
    });

    var html = '<div class="skills-panel">';
    var order = ['Combat', 'Crafting & Gathering', 'Agility & Stealth', 'Knowledge & Social', 'General'];
    order.forEach(function(cat) {
        var list = categories[cat];
        if (!list || list.length === 0) return;
        html += '<div class="skills-category"><div class="section-title">' + escapeHtml(cat) + '</div>';
        list.forEach(function(s) {
            var pct = s.xp_to_next > 0 ? Math.min((s.xp / s.xp_to_next) * 100, 100) : (s.rank >= 10 ? 100 : 0);
            var rankColor = s.rank === 0 ? 'var(--text-muted)' : s.rank >= 7 ? 'var(--text-gold-bright)' : s.rank >= 4 ? 'var(--text-gold)' : 'var(--text-light)';
            html += '<div class="skill-entry">';
            html += '<div class="skill-header">';
            html += '<span class="skill-name">' + escapeHtml(s.name) + '</span>';
            html += '<span class="skill-rank" style="color:' + rankColor + '">' + escapeHtml(s.rank_name) + ' (' + s.rank + ')</span>';
            html += '</div>';
            html += '<div class="bar-track skill-bar"><div class="bar-fill xp" style="width:' + pct + '%"></div></div>';
            html += '<div class="skill-xp-label">' + s.xp + ' / ' + s.xp_to_next + ' XP</div>';
            html += '</div>';
        });
        html += '</div>';
    });
    html += '</div>';
    el.innerHTML = html;
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
