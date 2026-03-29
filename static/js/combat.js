// Combat UI — BG3-style action bar and enemy status

export function renderCombatStarted(container, data) {
    const storyContent = container.querySelector('.story-content');
    if (!storyContent) return;

    // Initiative order bar
    const initBar = document.createElement('div');
    initBar.className = 'initiative-bar';
    initBar.id = 'initiativeBar';
    let html = '<div class="init-label">Initiative:</div>';
    data.initiative_order.forEach(entry => {
        const cls = entry.is_player ? 'init-entry player' : 'init-entry enemy';
        html += `<div class="${cls}" data-name="${entry.name}">${entry.name} (${entry.roll})</div>`;
    });
    initBar.innerHTML = html;

    // Insert at top of story content
    storyContent.insertBefore(initBar, storyContent.firstChild);
}

export function renderCombatTurnStart(container, data, onAction) {
    const storyContent = container.querySelector('.story-content');
    if (!storyContent) return;

    // Highlight current combatant in initiative bar
    document.querySelectorAll('.init-entry').forEach(el => el.classList.remove('active'));
    const activeInit = document.querySelector(`.init-entry[data-name="${data.combatant}"]`);
    if (activeInit) activeInit.classList.add('active');

    if (!data.is_player) return; // Enemy turns handled by CombatEnemyTurn

    // Remove any existing action bar
    document.getElementById('combatActionBar')?.remove();

    // Action bar
    const actionBar = document.createElement('div');
    actionBar.className = 'combat-action-bar';
    actionBar.id = 'combatActionBar';

    let barHtml = `<div class="action-economy">
        <span class="ae-badge action">Action: ${data.actions}</span>
        <span class="ae-badge bonus">Bonus: ${data.bonus_actions}</span>
        <span class="ae-badge move">Move: ${data.movement}ft</span>
        <span class="ae-round">Round ${data.round}</span>
    </div><div class="action-buttons">`;

    data.available_actions.forEach(action => {
        const icon = ACTION_ICONS[action.id] || '';
        const disabled = action.enabled ? '' : ' disabled';
        barHtml += `<button class="combat-btn${disabled}" data-action="${action.id}" title="${action.description}"${disabled}>
            <span class="combat-btn-icon">${icon}</span>
            <span class="combat-btn-label">${action.name}</span>
            <span class="combat-btn-cost">${action.cost}</span>
        </button>`;
    });
    barHtml += '</div>';

    // Enemy status
    barHtml += '<div class="combat-enemies">';
    data.enemies.filter(e => e.alive).forEach(enemy => {
        const hpPct = enemy.max_hp > 0 ? (Math.max(enemy.hp, 0) / enemy.max_hp * 100) : 0;
        const hpClass = hpPct > 50 ? '' : hpPct > 25 ? ' warning' : ' critical';
        barHtml += `<div class="combat-enemy-status">
            <span class="enemy-name-label">${enemy.name}</span>
            <div class="enemy-hp-bar"><div class="enemy-hp-fill${hpClass}" style="width:${hpPct}%"></div></div>
            <span class="enemy-hp-text">${enemy.hp}/${enemy.max_hp}</span>
        </div>`;
    });
    barHtml += '</div>';

    actionBar.innerHTML = barHtml;
    storyContent.appendChild(actionBar);
    storyContent.scrollTop = storyContent.scrollHeight;

    // Action button handlers
    actionBar.querySelectorAll('.combat-btn:not([disabled])').forEach(btn => {
        btn.addEventListener('click', () => {
            const actionId = btn.dataset.action;
            if (actionId === 'attack') {
                // Show target selector
                showTargetSelector(storyContent, data.enemies.filter(e => e.alive), (target) => {
                    onAction(actionId, target);
                });
            } else {
                onAction(actionId);
            }
        });
    });
}

function showTargetSelector(container, enemies, onSelect) {
    const existing = container.querySelector('.target-selector');
    if (existing) existing.remove();

    const div = document.createElement('div');
    div.className = 'target-selector';
    div.innerHTML = '<div class="target-prompt">Select target:</div>' +
        enemies.map(e => `<button class="target-btn" data-target="${e.name}">${e.name} (HP: ${e.hp}/${e.max_hp}, AC: ${e.ac})</button>`).join('');
    container.appendChild(div);
    container.scrollTop = container.scrollHeight;

    div.querySelectorAll('.target-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            div.remove();
            onSelect(btn.dataset.target);
        });
    });
}

export function renderCombatActionResult(container, data) {
    const storyContent = container.querySelector('.story-content');
    if (!storyContent) return;

    const div = document.createElement('div');
    const isHit = data.hit === true;
    const isMiss = data.hit === false;
    div.className = `combat-action-log ${isHit ? 'hit' : isMiss ? 'miss' : 'neutral'}`;
    div.textContent = data.description;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

export function renderEnemyTurn(container, data) {
    const storyContent = container.querySelector('.story-content');
    if (!storyContent) return;

    const div = document.createElement('div');
    div.className = `combat-action-log enemy ${data.hit ? 'hit' : 'miss'}`;
    let text = `${data.enemy_name} attacks with ${data.attack_name} (rolled ${data.attack_roll} vs AC ${data.target_ac}): `;
    text += data.hit ? `HIT for ${data.damage} damage! (HP: ${data.player_hp}/${data.player_max_hp})` : 'MISS!';
    div.textContent = text;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

export function renderCombatEnded(container, data) {
    const storyContent = container.querySelector('.story-content');
    if (!storyContent) return;

    // Remove action bar
    document.getElementById('combatActionBar')?.remove();

    const div = document.createElement('div');
    div.className = `combat-ended ${data.victory ? 'victory' : 'defeat'}`;
    div.innerHTML = data.victory
        ? `<div class="combat-ended-title">Victory!</div><div>+${data.xp_reward} XP</div>`
        : `<div class="combat-ended-title">Defeated...</div>`;
    storyContent.appendChild(div);
    storyContent.scrollTop = storyContent.scrollHeight;
}

const ACTION_ICONS = {
    'attack': '\u2694\uFE0F',
    'dodge': '\uD83D\uDEE1\uFE0F',
    'dash': '\uD83D\uDCA8',
    'use_item': '\uD83E\uDDEA',
    'second_wind': '\u2764\uFE0F',
    'cunning_hide': '\uD83D\uDC7B',
    'healing_word': '\u2728',
    'end_turn': '\u23ED\uFE0F',
};
