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

const HEARTBEAT_DELAY = 10_000;
const LATEHEARTBEAT_DELAY = 10_000;

class Socket {
	#closed;
	#callbacks;
	#pings;
	#heart;
	#lastHeartbeat;

	constructor(url = null) {
		if (!url) {
			if (window.location.protocol === "https:") {
				this.url = "wss://" + window.location.host + "/socket";
			} else {
				this.url = "ws://" + window.location.host + "/socket";
			}
		} else {
			this.url = url;
		}

		this.#closed = false;
		this.#callbacks = [];

		this.socket = null;

		this.#initSocket();
	}
	#initSocket() {
		this.#pings = [];
		this.socket = new WebSocket(this.url);
		this.socket.addEventListener("open",    ()         => this.#handleOpen());
		this.socket.addEventListener("message", (response) => this.#handleMessage(response));
		this.socket.addEventListener("error",   (error)    => this.#handleError(error));
		this.socket.addEventListener("close",   ()         => this.#handleClose());
	}
	#heartbeat() {
		this.socket.send("heartbeat");
	}
	#handleOpen() {
			if (this.socket.readyState !== 1) {
				return;
			}

			if (this.#heart !== null) {
				clearInterval(this.#heart);
				this.#heart = null;
			}

			// regularly poll the server to make sure we're still connected
			this.#lastHeartbeat = performance.now();
			this.#heart = setInterval(() => {
				if (performance.now() - this.#lastHeartbeat >= HEARTBEAT_DELAY) {
					// grace period if we haven't received a response
					setTimeout(() => {
						if (performance.now() - this.#lastHeartbeat >= LATEHEARTBEAT_DELAY) {
							clearInterval(this.#heart);
							this.#heart = null;
							this.#initSocket();
						}
					}, LATEHEARTBEAT_DELAY);
				} else {
					this.#heartbeat();
				}
			}, HEARTBEAT_DELAY);

			this.#heartbeat();

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
		if (!this.#closed) {
			this.#initSocket();
		}
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
		this.#pings.push(performance.now());
		this.socket.send("ping");
	}
	emit(name, data) {
		this.socket.send(new ClientRequest(name, data).toString());
	}
	close(reason) {
		this.socket.close(1000, reason);
		this.#closed = true;
	}
}

export { Socket as default };
