// Auth and API helpers

export function getToken() {
    return localStorage.getItem('rq_token');
}

export function setToken(token) {
    localStorage.setItem('rq_token', token);
}

export function clearToken() {
    localStorage.removeItem('rq_token');
    localStorage.removeItem('rq_username');
}

function authHeaders(extra = {}) {
    const token = getToken();
    return {
        'Content-Type': 'application/json',
        ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
        ...extra,
    };
}

async function authFetch(url, options = {}) {
    const res = await fetch(url, {
        ...options,
        headers: authHeaders(options.headers),
    });
    if (res.status === 401) {
        clearToken();
        window.location.href = '/login';
        throw new Error('Unauthorized');
    }
    return res;
}

export async function getMe() {
    const res = await authFetch('/api/auth/me');
    if (!res.ok) throw new Error('Not authenticated');
    return res.json();
}

export function getWsUrl() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const token = getToken();
    return `${protocol}//${window.location.host}/ws?token=${token}`;
}
