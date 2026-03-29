// Adventure selection and character creation screens

const SCENARIO_PRESETS = {
    '': { name: 'Random', desc: 'Let the DM surprise you' },
    'lost-dungeon': { name: 'The Lost Dungeon', desc: 'Explore ancient ruins deep underground, filled with traps, puzzles, and forgotten treasure' },
    'dragons-lair': { name: "Dragon's Lair", desc: "Journey to the volcanic mountains to face a fearsome dragon terrorizing nearby villages" },
    'city-intrigue': { name: 'City Intrigue', desc: 'Navigate political conspiracies, thieves guilds, and noble machinations in a sprawling metropolis' },
    'wilderness-survival': { name: 'Wilderness Survival', desc: 'Brave untamed forests, cross treacherous mountains, and survive hostile wildlife on an expedition' },
    'haunted-manor': { name: 'Haunted Manor', desc: 'Investigate a cursed noble estate where the dead refuse to rest and dark secrets lurk in every shadow' },
};

export function renderSelectScreen(container, adventures, handlers) {
    let html = `<div class="select-screen">
        <h1>RuneQuest</h1>
        <div class="decorative-line"></div>`;

    if (adventures && adventures.length > 0) {
        html += '<div class="adventures-grid">';
        adventures.forEach(a => {
            html += `<div class="adventure-card" data-id="${a.id}">
                <div class="card-name">${escapeHtml(a.character_name)}</div>
                <div class="card-details">
                    ${escapeHtml(a.race)} ${escapeHtml(a.class)} &middot; Level ${a.level}<br>
                    <em>${escapeHtml(a.name)}</em>
                </div>
            </div>`;
        });
        html += '</div>';
    } else {
        html += '<div class="empty-state">No adventures yet. Create your first one!</div>';
    }

    html += `<button class="stone-btn" id="newAdventureBtn">New Adventure</button>
        <button class="stone-btn danger" id="logoutBtn" style="margin-top:12px;">Log Out</button>
    </div>`;

    container.innerHTML = html;

    container.querySelectorAll('.adventure-card').forEach(card => {
        card.addEventListener('click', () => handlers.onLoad(card.dataset.id));
    });
    document.getElementById('newAdventureBtn')?.addEventListener('click', handlers.onNew);
    document.getElementById('logoutBtn')?.addEventListener('click', () => {
        localStorage.removeItem('rq_token');
        localStorage.removeItem('rq_username');
        window.location.href = '/login';
    });
}

export function renderCreateScreen(container, handlers) {
    let scenarioCardsHtml = '';
    for (const [key, preset] of Object.entries(SCENARIO_PRESETS)) {
        const selected = key === '' ? ' selected' : '';
        scenarioCardsHtml += `<div class="scenario-card${selected}" data-scenario="${key}">
            <div class="scenario-name">${escapeHtml(preset.name)}</div>
            <div class="scenario-desc">${escapeHtml(preset.desc)}</div>
        </div>`;
    }

    container.innerHTML = `
    <div class="select-screen" style="overflow-y: auto;">
        <button class="btn-back" id="backBtn">&larr; Back</button>
        <div class="create-screen">
            <h2>Create Your Adventurer</h2>
            <div class="decorative-line"></div>
            <form class="create-form" id="createForm">
                <div class="form-group">
                    <label>Adventure Name</label>
                    <input type="text" id="advName" placeholder="The Lost Mines of Phandelver" required>
                </div>
                <div class="form-group">
                    <label>Character Name</label>
                    <input type="text" id="charName" placeholder="Aragorn" required>
                </div>
                <div class="form-group">
                    <label>Race</label>
                    <select id="charRace">
                        <option value="human">Human</option>
                        <option value="elf">Elf</option>
                        <option value="dwarf">Dwarf</option>
                        <option value="orc">Orc</option>
                        <option value="halfling">Halfling</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Class</label>
                    <select id="charClass">
                        <option value="warrior">Warrior</option>
                        <option value="mage">Mage</option>
                        <option value="rogue">Rogue</option>
                        <option value="cleric">Cleric</option>
                        <option value="ranger">Ranger</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Scenario</label>
                    <div class="scenario-cards" id="scenarioCards">${scenarioCardsHtml}</div>
                    <textarea id="customScenario" placeholder="Or describe your own scenario..." rows="3"></textarea>
                </div>
                <div class="form-group">
                    <label>Stats (Point Buy - 27 Points)</label>
                    <div class="points-remaining">Points remaining: <strong id="pointsLeft">27</strong></div>
                    <div class="stat-allocator">
                        <div class="stat-alloc-box">
                            <div class="alloc-name">STR</div>
                            <input type="number" id="statStr" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costStr">Cost: 2</div>
                        </div>
                        <div class="stat-alloc-box">
                            <div class="alloc-name">DEX</div>
                            <input type="number" id="statDex" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costDex">Cost: 2</div>
                        </div>
                        <div class="stat-alloc-box">
                            <div class="alloc-name">CON</div>
                            <input type="number" id="statCon" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costCon">Cost: 2</div>
                        </div>
                        <div class="stat-alloc-box">
                            <div class="alloc-name">INT</div>
                            <input type="number" id="statInt" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costInt">Cost: 2</div>
                        </div>
                        <div class="stat-alloc-box">
                            <div class="alloc-name">WIS</div>
                            <input type="number" id="statWis" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costWis">Cost: 2</div>
                        </div>
                        <div class="stat-alloc-box">
                            <div class="alloc-name">CHA</div>
                            <input type="number" id="statCha" min="8" max="15" value="10" class="stat-input">
                            <div class="stat-cost" id="costCha">Cost: 2</div>
                        </div>
                    </div>
                </div>
                <button type="submit" class="stone-btn" style="width: 100%; margin-top: 16px;">Begin Adventure</button>
            </form>
        </div>
    </div>`;

    document.getElementById('backBtn')?.addEventListener('click', handlers.onBack);

    // Scenario card selection
    document.querySelectorAll('.scenario-card').forEach(card => {
        card.addEventListener('click', () => {
            document.querySelectorAll('.scenario-card').forEach(c => c.classList.remove('selected'));
            card.classList.add('selected');
        });
    });

    // Point buy calculator
    const pointCost = (val) => {
        if (val <= 13) return val - 8;
        if (val === 14) return 7;
        if (val === 15) return 9;
        return 0;
    };

    const statIds = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha'];

    const updatePoints = () => {
        let used = 0;
        statIds.forEach(id => {
            const inp = document.getElementById(`stat${id}`);
            const val = parseInt(inp.value) || 8;
            const cost = pointCost(val);
            used += cost;
            const costEl = document.getElementById(`cost${id}`);
            if (costEl) {
                let text = `Cost: ${cost}`;
                if (val < 15) {
                    const nextCost = pointCost(val + 1) - cost;
                    text += ` (+${nextCost} next)`;
                }
                costEl.textContent = text;
            }
        });
        const left = 27 - used;
        document.getElementById('pointsLeft').textContent = left;
        document.getElementById('pointsLeft').style.color = left < 0 ? '#cc4444' : '#c8a84e';
    };

    document.querySelectorAll('.stat-input').forEach(inp => {
        inp.addEventListener('input', updatePoints);
    });
    updatePoints();

    document.getElementById('createForm')?.addEventListener('submit', (e) => {
        e.preventDefault();
        const pointsLeft = parseInt(document.getElementById('pointsLeft').textContent);
        if (pointsLeft < 0) {
            alert('You have spent too many points!');
            return;
        }

        // Determine scenario
        const customScenario = document.getElementById('customScenario').value.trim();
        let scenario = customScenario;
        if (!scenario) {
            const selectedCard = document.querySelector('.scenario-card.selected');
            const key = selectedCard?.dataset.scenario || '';
            if (key && SCENARIO_PRESETS[key]) {
                scenario = SCENARIO_PRESETS[key].desc;
            }
        }

        handlers.onCreate({
            name: document.getElementById('advName').value.trim(),
            character_name: document.getElementById('charName').value.trim(),
            race: document.getElementById('charRace').value,
            class: document.getElementById('charClass').value,
            scenario: scenario || undefined,
            stats: {
                strength: parseInt(document.getElementById('statStr').value),
                dexterity: parseInt(document.getElementById('statDex').value),
                constitution: parseInt(document.getElementById('statCon').value),
                intelligence: parseInt(document.getElementById('statInt').value),
                wisdom: parseInt(document.getElementById('statWis').value),
                charisma: parseInt(document.getElementById('statCha').value),
            },
        });
    });
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}
