// Adventure selection and character creation screens

export function renderSelectScreen(container, adventures, handlers) {
    let html = `<div class="select-screen">
        <h1>RuneQuest</h1>
        <div class="decorative-line"></div>`;

    if (adventures && adventures.length > 0) {
        html += '<div class="adventures-grid">';
        adventures.forEach(a => {
            const bg = a.background || 'Adventurer';
            html += `<div class="adventure-card" data-id="${a.id}">
                <div class="card-name">${escapeHtml(a.character_name)}</div>
                <div class="card-details">
                    ${escapeHtml(a.race)} ${escapeHtml(bg)}<br>
                    <em>${escapeHtml(a.name)}</em>
                </div>
            </div>`;
        });
        html += '</div>';
    } else {
        html += '<div class="empty-state">No adventures yet. Create your first one!</div>';
    }

    html += `<button class="stone-btn" id="newAdventureBtn">New Adventure</button>
        <button class="stone-btn" id="accountBtn" style="margin-top:12px;">Account Settings</button>
        <button class="stone-btn danger" id="logoutBtn" style="margin-top:12px;">Log Out</button>
    </div>`;

    container.innerHTML = html;

    container.querySelectorAll('.adventure-card').forEach(card => {
        card.addEventListener('click', () => handlers.onLoad(card.dataset.id));
    });
    document.getElementById('newAdventureBtn')?.addEventListener('click', handlers.onNew);
    document.getElementById('accountBtn')?.addEventListener('click', () => {
        if (handlers.onAccount) handlers.onAccount();
    });
    document.getElementById('logoutBtn')?.addEventListener('click', () => {
        localStorage.removeItem('rq_token');
        localStorage.removeItem('rq_username');
        window.location.href = '/login';
    });
}

export function renderCreateScreen(container, handlers) {
    container.innerHTML = `
    <div class="select-screen" style="overflow-y: auto;">
        <button class="btn-back" id="backBtn">&larr; Back</button>
        <div class="create-screen">
            <h2>Create Your Adventurer</h2>
            <div class="decorative-line"></div>
            <form class="create-form" id="createForm">
                <div class="form-group">
                    <label>Adventure Name</label>
                    <input type="text" id="advName" placeholder="A New Beginning" required>
                </div>
                <div class="form-group">
                    <label>Character Name</label>
                    <input type="text" id="charName" placeholder="Your character's name" required>
                </div>
                <div class="form-group">
                    <label>Race <span style="color:#888;font-size:11px">(determines starting region)</span></label>
                    <select id="charRace">
                        <option value="human">Human — Versatile, starts in the Southern Heartlands</option>
                        <option value="dwarf">Dwarf — Hardy mountain folk, starts in the Northwest</option>
                        <option value="elf">Elf — Agile forest dwellers, starts in the Southwest</option>
                        <option value="orc">Orc — Strong frontier warriors, starts in the Northeast</option>
                        <option value="halfling">Halfling — Charming coastal folk, starts in the Southeast</option>
                        <option value="gnome">Gnome — Inventive tinkers, starts in the Western Hills</option>
                        <option value="dragonborn">Dragonborn — Draconic heritage, starts in the Northern Peaks</option>
                        <option value="faefolk">Faefolk — Magical nature spirits, starts in the Eastern Glens</option>
                        <option value="goblin">Goblin — Cunning survivors, starts in the Southern Swamps</option>
                        <option value="revenant">Revenant — The undying, starts in the Northern Blight</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Background <span style="color:#888;font-size:11px">(determines starting skills & equipment)</span></label>
                    <select id="charBackground">
                        <option value="farmhand">Farmhand — Fortitude + Leatherworking (starts with spear)</option>
                        <option value="apprentice_smith">Apprentice Smith — Smithing + Weapon Mastery (starts with hammer)</option>
                        <option value="street_urchin">Street Urchin — Stealth + Lockpicking (starts with dagger)</option>
                        <option value="hunter">Hunter — Marksmanship + Tracking (starts with bow)</option>
                        <option value="acolyte">Acolyte — Healing + Blessing (starts with staff)</option>
                        <option value="scholar">Scholar — Lore + Enchanting (starts with tome)</option>
                        <option value="merchant">Merchant — Charm + Inspire (starts with 20 gold)</option>
                        <option value="herbalist">Herbalist — Alchemy + Survival (starts with herbs)</option>
                        <option value="woodcutter">Woodcutter — Woodworking + Fortitude (starts with axe)</option>
                        <option value="drifter">Drifter — Nothing. Hard mode.</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Backstory <span style="color:#888;font-size:11px">(optional — the DM will weave this into your story)</span></label>
                    <textarea id="charBackstory" placeholder="A disgraced merchant seeking redemption in a new land..." rows="3"></textarea>
                </div>
                <button type="submit" class="stone-btn" style="width: 100%; margin-top: 16px;">Begin Adventure</button>
            </form>
        </div>
    </div>`;

    document.getElementById('backBtn')?.addEventListener('click', handlers.onBack);

    document.getElementById('createForm')?.addEventListener('submit', (e) => {
        e.preventDefault();

        handlers.onCreate({
            name: document.getElementById('advName').value.trim(),
            character_name: document.getElementById('charName').value.trim(),
            race: document.getElementById('charRace').value,
            background: document.getElementById('charBackground').value,
            backstory: document.getElementById('charBackstory').value.trim() || undefined,
        });
    });
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
}
