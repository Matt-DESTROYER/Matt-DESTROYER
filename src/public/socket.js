class ClientRequest {
	constructor(name, data) {
		this.name = name;
		this.data = data;
		this.timeStamp = Date.now();
	}
	toString() {
		return JSON.stringify(this);
	}
}

class Socket {
	#closed;
	#callbacks;
	#pings;
	#socket;
	#heartbeating;

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
		this.#initSocket();
	}
	async #initSocket() {
		this.#pings = [];
		this.connected = false;
		this.#socket = new WebSocket(this.url);
		let heartbeat;
		this.#socket.addEventListener("open", () => {
			if (this.#socket.readyState === 1) {
				this.connected = true;
				this.#heartbeating = true;
				heartbeat = setInterval(() => {
					this.#heartbeat();
					setTimeout(() => {
						if (!this.#heartbeating) {
							clearInterval(heartbeat);
							console.info("[Socket.js] Disconnected from " + this.url + ".");
							this.#initSocket();
						}
					}, 1500);
				}, 2000);
			}
			for (let i = 0; i < this.#callbacks.length; i++) {
				if (this.#callbacks[i].name === "connection" ||
						this.#callbacks[i].name === "connect") {
					this.#callbacks[i].callback();
					if (this.#callbacks[i].once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
		});
		this.#socket.addEventListener("message", (response) => {
			if (response.data === "pong") {
				const ping = Date.now() - this.#pings.shift();
				for (let i = 0; i < this.#callbacks.length; i++) {
					if (this.#callbacks[i].name === "pong") {
						this.#callbacks[i].callback(ping);
						if (this.#callbacks[i].once) {
							this.#callbacks.splice(i, 1);
							i--;
						}
					}
				}
				return;
			} else if (response.data === "heartbeat") {
				this.#heartbeating = true;
				return;
			}
			const { name, data: _data } = JSON.parse(response.data);
			for (let i = 0; i < this.#callbacks.length; i++) {
				if (this.#callbacks[i].name === name) {
					this.#callbacks[i].callback(_data);
					if (this.#callbacks[i].once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
		});
		this.#socket.addEventListener("error", (err) => {
			for (let i = 0; i < this.#callbacks.length; i++) {
				if (this.#callbacks[i].name === "error") {
					const { once } = this.#callbacks[i];
					this.#callbacks[i].callback(err);
					if (this.#callbacks[i].once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
		});
		this.#socket.addEventListener("close", () => {
			this.connected = false;
			if (!this.#closed) {
				clearInterval(heartbeat);
				console.log("[Socket.js] Disconnected from " + this.url + ".");
				this.#initSocket();
			}
			for (let i = 0; i < this.#callbacks.length; i++) {
				if (this.#callbacks[i].name === "disconnect") {
					this.#callbacks[i].callback();
					if (this.#callbacks[i].once) {
						this.#callbacks.splice(i, 1);
						i--;
					}
				}
			}
		});
	}
	async #heartbeat() {
		if (this.connected) {
			if (this.#socket.readyState === 1) {
				this.connected = true;
				this.#heartbeating = false;
				this.#socket.send("heartbeat");
			} else {
				this.connected = false;
				this.#heartbeating = false;
			}
		}
	}
	async once(name, callback) {
		this.#callbacks.push({ name, callback: callback.bind(this), once: true });
	}
	async on(name, callback) {
		this.#callbacks.push({ name, callback: callback.bind(this), once: false });
	}
	async ping() {
		if (this.connected) {
			this.#pings.push(Date.now());
			this.#socket.send("ping");
		}
	}
	async emit(name, data) {
		if (this.connected) {
			this.#socket.send(new ClientRequest(name, data).toString());
		}
	}
	async close(reason) {
		this.#socket.close(1000, reason);
		this.#closed = true;
	}
}

export { Socket as default };
