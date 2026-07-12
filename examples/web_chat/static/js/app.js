const elements = {
    form: document.querySelector('#chatForm'),
    input: document.querySelector('#messageInput'),
    send: document.querySelector('#sendButton'),
    messages: document.querySelector('#messages'),
    newChat: document.querySelector('#newChatButton'),
    theme: document.querySelector('#themeButton'),
    status: document.querySelector('#connectionStatus'),
    statusText: document.querySelector('#connectionText'),
    think: document.querySelector('#thinkInput'),
    temperature: document.querySelector('#temperatureInput'),
    temperatureOutput: document.querySelector('#temperatureOutput'),
    maxTokens: document.querySelector('#maxTokensInput'),
    maxTokensOutput: document.querySelector('#maxTokensOutput'),
    characterCount: document.querySelector('#characterCount'),
};

const state = {
    sessionId: null,
    busy: false,
};

const MAX_STREAM_EVENT_CHARS = 1_100_000;
const MAX_RESPONSE_CHARS = 1_048_576;
const MAX_ERROR_CHARS = 512;

function setTheme(theme) {
    document.documentElement.dataset.theme = theme;
    try {
        localStorage.setItem('zai-web-chat-theme', theme);
    } catch {
        // Storage can be disabled by browser privacy settings; the in-memory
        // theme still works for the current page.
    }
    elements.theme.textContent = theme === 'dark' ? 'Light theme' : 'Dark theme';
}

function initializeTheme() {
    let saved = null;
    try {
        saved = localStorage.getItem('zai-web-chat-theme');
    } catch {
        // Fall back to the operating-system preference when storage is denied.
    }
    const preferred = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    setTheme(saved === 'dark' || saved === 'light' ? saved : preferred);
}

function setBusy(busy) {
    state.busy = busy;
    elements.input.disabled = busy;
    elements.newChat.disabled = busy;
    elements.send.disabled = busy || elements.input.value.trim().length === 0;
}

function updateComposer() {
    const length = elements.input.value.length;
    elements.characterCount.textContent = `${length} / 10000`;
    elements.send.disabled = state.busy || elements.input.value.trim().length === 0;
}

function addMessage(role, content, pending = false) {
    document.querySelector('#welcome')?.remove();

    const article = document.createElement('article');
    article.className = `message ${role}${pending ? ' pending' : ''}`;

    const avatar = document.createElement('div');
    avatar.className = 'message-avatar';
    avatar.textContent = role === 'user' ? 'U' : 'AI';
    avatar.setAttribute('aria-hidden', 'true');

    const body = document.createElement('div');
    body.className = 'message-body';
    const label = document.createElement('span');
    label.className = 'message-role';
    label.textContent = role === 'user' ? 'You' : 'Assistant';
    const text = document.createElement('p');
    text.className = 'message-text';
    text.textContent = content;

    body.append(label, text);
    article.append(avatar, body);
    elements.messages.append(article);
    elements.messages.scrollTo({ top: elements.messages.scrollHeight, behavior: 'smooth' });
    return { article, text };
}

function resetConversation() {
    state.sessionId = null;
    elements.messages.replaceChildren();
    const welcome = document.createElement('div');
    welcome.className = 'welcome';
    welcome.id = 'welcome';
    const title = document.createElement('h2');
    title.textContent = 'Start a conversation';
    const copy = document.createElement('p');
    copy.textContent = 'Messages are kept in memory for this server process only.';
    welcome.append(title, copy);
    elements.messages.append(welcome);
    elements.input.focus();
}

async function readError(response) {
    try {
        const body = await response.json();
        return typeof body?.error?.message === 'string'
            && body.error.message.length <= MAX_ERROR_CHARS
            ? body.error.message
            : `HTTP ${response.status}`;
    } catch {
        return `HTTP ${response.status}`;
    }
}

function parseStreamChunk(data) {
    let chunk;
    try {
        chunk = JSON.parse(data);
    } catch {
        throw new Error('The server sent an invalid stream event.');
    }
    if (
        !chunk
        || typeof chunk !== 'object'
        || Array.isArray(chunk)
        || typeof chunk.session_id !== 'string'
        || !/^[A-Za-z0-9_-]{1,128}$/.test(chunk.session_id)
        || typeof chunk.content !== 'string'
        || typeof chunk.done !== 'boolean'
        || (chunk.error != null && typeof chunk.error !== 'string')
    ) {
        throw new Error('The server sent an invalid stream event.');
    }
    if (
        chunk.error != null
        && (!chunk.done || chunk.error.length === 0 || chunk.error.length > MAX_ERROR_CHARS)
    ) {
        throw new Error('The server sent an invalid stream event.');
    }
    return chunk;
}

async function consumeSse(response, onChunk) {
    const contentType = response.headers.get('content-type') || '';
    if (!/^text\/event-stream(?:\s*;|\s*$)/i.test(contentType)) {
        throw new Error('The server did not return an event stream.');
    }
    if (!response.body) {
        throw new Error('The browser did not expose a response stream.');
    }
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    const processBlock = (block) => {
        if (block.length > MAX_STREAM_EVENT_CHARS) {
            throw new Error('The server sent an oversized stream event.');
        }
        const data = block
            .split(/\r?\n/)
            .filter((line) => line.startsWith('data:'))
            .map((line) => line.slice(5).trimStart())
            .join('\n');
        if (data) onChunk(parseStreamChunk(data));
    };

    try {
        while (true) {
            const { done, value } = await reader.read();
            buffer += decoder.decode(value || new Uint8Array(), { stream: !done });
            const blocks = buffer.split(/\r?\n\r?\n/);
            buffer = blocks.pop() || '';
            if (buffer.length > MAX_STREAM_EVENT_CHARS) {
                throw new Error('The server sent an oversized stream event.');
            }
            blocks.forEach(processBlock);
            if (done) break;
        }
        if (buffer.trim()) processBlock(buffer);
    } catch (error) {
        await reader.cancel(error).catch(() => {});
        throw error;
    } finally {
        reader.releaseLock();
    }
}

async function sendMessage(message) {
    addMessage('user', message);
    const assistant = addMessage('assistant', 'Waiting for the model…', true);
    setBusy(true);

    try {
        const response = await fetch('/api/chat/stream', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Accept: 'text/event-stream' },
            body: JSON.stringify({
                message,
                session_id: state.sessionId,
                think: elements.think.checked,
                temperature: Number(elements.temperature.value),
                max_tokens: Number(elements.maxTokens.value),
            }),
        });
        if (!response.ok) throw new Error(await readError(response));

        let content = '';
        let terminalSeen = false;
        let streamError = null;
        let responseSessionId = state.sessionId;
        await consumeSse(response, (chunk) => {
            if (terminalSeen) {
                throw new Error('The server sent data after the terminal event.');
            }
            if (responseSessionId && chunk.session_id !== responseSessionId) {
                throw new Error('The server changed sessions during the response.');
            }
            responseSessionId = chunk.session_id;
            state.sessionId = chunk.session_id;
            if (chunk.error) {
                streamError = chunk.error;
                terminalSeen = true;
                return;
            }
            if (content.length + chunk.content.length > MAX_RESPONSE_CHARS) {
                throw new Error('The response exceeded the browser limit.');
            }
            content += chunk.content;
            assistant.text.textContent = content || 'The provider returned an empty response.';
            terminalSeen = chunk.done;
            elements.messages.scrollTop = elements.messages.scrollHeight;
        });
        if (streamError) throw new Error(streamError);
        if (!terminalSeen) throw new Error('The response stream ended unexpectedly.');
        assistant.article.classList.remove('pending');
    } catch (error) {
        assistant.article.classList.remove('pending');
        assistant.article.classList.add('error');
        const message = error instanceof Error ? error.message : 'Unknown request error.';
        assistant.text.textContent = `Request failed: ${message}`;
    } finally {
        setBusy(false);
        elements.input.focus();
    }
}

async function checkHealth() {
    try {
        const response = await fetch('/health', { headers: { Accept: 'application/json' } });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        elements.status.classList.add('online');
        elements.status.classList.remove('offline');
        elements.statusText.textContent = 'Server ready';
    } catch {
        elements.status.classList.add('offline');
        elements.status.classList.remove('online');
        elements.statusText.textContent = 'Server unavailable';
    }
}

elements.form.addEventListener('submit', (event) => {
    event.preventDefault();
    const message = elements.input.value.trim();
    if (!message || state.busy) return;
    elements.input.value = '';
    updateComposer();
    void sendMessage(message);
});

elements.input.addEventListener('input', updateComposer);
elements.input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
        event.preventDefault();
        elements.form.requestSubmit();
    }
});
elements.newChat.addEventListener('click', resetConversation);
elements.theme.addEventListener('click', () => {
    setTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark');
});
elements.temperature.addEventListener('input', () => {
    elements.temperatureOutput.value = elements.temperature.value;
});
elements.maxTokens.addEventListener('input', () => {
    elements.maxTokensOutput.value = elements.maxTokens.value;
});

initializeTheme();
updateComposer();
void checkHealth();
