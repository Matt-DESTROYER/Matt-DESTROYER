class ClientRequest {
	constructor(name, data) {
		this.name = name;
		this.data = data;
		this.timeStamp = performance.now();
	}
	toString() {
		return JSON.stringify(this);
	}
}

const HEARTBEAT_DELAY     = 2_000;
const LATEHEARTBEAT_DELAY = 250;
const RECONNECT_DELAY     = 200;
const RECONNECT_DELAY_MAX = 10_000;

class Socket {
	#closed;
	#callbacks;
	#pings;
	#heart;
	#lastHeartbeat;
	#reconnectDelay;

	constructor(url = null) {
		if (!url) {
			if (window.location.protocol === "https:") {
				this.url = "wss://" + window.location.host + "/websocket";
			} else {
				this.url = "ws://" + window.location.host + "/websocket";
			}
		} else {
			this.url = url;
		}

		this.#closed = false;
		this.#callbacks = [];

		this.socket = null;

		this.#initSocket();
	}
	get readyState() {
		return this.socket ? this.socket.readyState : null;
	}
	#initSocket() {
		if (this.socket && (
			this.readyState === WebSocket.CONNECTING ||
			this.readyState === WebSocket.OPEN
		)) {
			try {
				this.socket.close(1000, "Reconnecting");
			} catch (_) {}
		}

		if (this.#heart !== null) {
			clearInterval(this.#heart);
			this.#heart = null;
		}

		this.#pings = [];
		this.socket = new WebSocket(this.url);
		this.socket.addEventListener("open",    ()         => this.#handleOpen());
		this.socket.addEventListener("message", (response) => this.#handleMessage(response));
		this.socket.addEventListener("error",   (error)    => this.#handleError(error));
		this.socket.addEventListener("close",   ()         => this.#handleClose());
	}
	#scheduleHeartbeat() {
		if (this.#heart !== null) {
			clearTimeout(this.#heart);
			this.#heart = null;
		}
		this.#heart = setTimeout(() => {
			if (performance.now() - this.#lastHeartbeat >= HEARTBEAT_DELAY + LATEHEARTBEAT_DELAY) {
				this.#scheduleReconnect();
				return;
			}

			if (this.readyState === WebSocket.OPEN) {
				this.socket.send("heartbeat");
			}

			this.#scheduleHeartbeat();
		}, HEARTBEAT_DELAY);
	}
	#scheduleReconnect() {
		console.warn("Reconnecting to the server...");
		if (this.#closed) return;
		setTimeout(() => this.#initSocket(), this.#reconnectDelay);
		this.#reconnectDelay = Math.min(this.#reconnectDelay * 2, RECONNECT_DELAY_MAX)
	}
	#handleOpen() {
		if (this.readyState !== WebSocket.OPEN) return;

		if (this.#heart !== null) {
			clearInterval(this.#heart);
			this.#heart = null;
		}

		// regularly poll the server to make sure we're still connected
		this.#reconnectDelay = RECONNECT_DELAY;
		this.#lastHeartbeat = performance.now();
		this.socket.send("heartbeat");
		this.#scheduleHeartbeat();

		for (let i = 0; i < this.#callbacks.length; i++) {
			const callback = this.#callbacks[i];
			if (callback.name === "connection" ||
					callback.name === "connect") {
				callback.callback();
				if (callback.once) {
					this.#callbacks.splice(i, 1);
					i--;
				}
			}
		}
	}
	#handleMessage(response) {
		if (response.data === "pong") {
			const ping = performance.now() - this.#pings.shift();
			for (let i = 0; i < this.#callbacks.length; i++) {
				const callback = this.#callbacks[i];
				if (callback.name === "pong") {
					callback.callback(ping);
					if (callback.once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
			return;
		}

		if (response.data === "heartbeat") {
			this.#lastHeartbeat = performance.now();
			return;
		}
	
		// treat as regular message
		try {
			const { name, data } = JSON.parse(response.data);
			for (let i = 0; i < this.#callbacks.length; i++) {
				const callback = this.#callbacks[i];
				if (callback.name === name) {
					callback.callback(data);
					if (callback.once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
		} catch (err) {
			console.warn("Invalid JSON from server:", response.data);
		}
	}
	#handleError(error) {
		for (let i = 0; i < this.#callbacks.length; i++) {
			const callback = this.#callbacks[i];
			if (callback.name === "error") {
				callback.callback(error);
				if (callback.once) {
					this.#callbacks.splice(i, 1);
					i--;
				}
			}
		}
	}
	#handleClose() {
		for (let i = 0; i < this.#callbacks.length; i++) {
			const callback = this.#callbacks[i];
			if (callback.name === "disconnect") {
				callback.callback();
				if (callback.once) {
					this.#callbacks.splice(i, 1);
					i--;
				}
			}
		}
	}
	once(name, callback) {
		this.#callbacks.push({ name, callback: callback.bind(this), once: true });
	}
	on(name, callback) {
		this.#callbacks.push({ name, callback: callback.bind(this), once: false });
	}
	ping() {
		if (this.readyState === WebSocket.OPEN) {
			this.#pings.push(performance.now());
			this.socket.send("ping");
		}
	}
	emit(name, data) {
		if (this.readyState === WebSocket.OPEN) {
			this.socket.send(new ClientRequest(name, data).toString());
		}
	}
	close(reason) {
		if (this.socket && (
			this.readyState === WebSocket.CONNECTING ||
			this.readyState === WebSocket.OPEN
		)) {
			this.socket.close(1000, reason);
		}
		if (this.#heart !== null) {
			clearInterval(this.#heart);
			this.#heart = null;
		}
		this.#closed = true;
	}
}

export { Socket as default };
