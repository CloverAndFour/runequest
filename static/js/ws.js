// WebSocket manager with auto-reconnect

export class WebSocketManager {
    constructor(url, handlers) {
        this.url = url;
        this.handlers = handlers;
        this.ws = null;
        this.reconnectDelay = 1000;
        this.maxReconnectDelay = 30000;
        this.shouldReconnect = true;
    }

    connect() {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            this.reconnectDelay = 1000;
            if (this.handlers.onOpen) this.handlers.onOpen();
        };

        this.ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                if (this.handlers.onMessage) this.handlers.onMessage(msg);
            } catch (e) {
                console.error('Failed to parse WS message:', e);
            }
        };

        this.ws.onclose = () => {
            if (this.shouldReconnect) {
                setTimeout(() => this.connect(), this.reconnectDelay);
                this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
            }
            if (this.handlers.onClose) this.handlers.onClose();
        };

        this.ws.onerror = (err) => {
            if (this.handlers.onError) this.handlers.onError(err);
        };
    }

    send(msg) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(msg));
        }
    }

    close() {
        this.shouldReconnect = false;
        if (this.ws) this.ws.close();
    }
}
