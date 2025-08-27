const CHANNELS = new Map();
const SOCKETS = [];
const CALLBACKS = [];

class ServerResponse {
	constructor(name, data) {
		this.name = name;
		this.data = data;
		this.timeStamp = Date.now();
	}
	toString() {
		return JSON.stringify(this);
	}
}

class Channel {
	#callbacks;

	constructor(name) {
		this.name = name;
		this.sockets = [];
		this.#callbacks = [];
		CHANNELS.set(this.name, this);
	}
	get connected() {
		return this.sockets.length;
	}
	on(name, callback) {
		callback = callback.bind(this);
		this.#callbacks.push({ name, callback });
	}
	add(socket) {
		if (!this.sockets.includes(socket)) {
			this.sockets.push(socket);
			for (const callback of this.#callbacks) {
				if (callback.name === "connection" ||
						callback.name  === "connect") {
					callback.callback(socket);
				}
			}
		}
	}
	remove(socket) {
		const idx = this.sockets.indexOf(socket);
		if (idx !== -1) {
			this.sockets.splice(idx, 1);
			for (const callback of this.#callbacks) {
				if (callback.name === "disconnect") {
					callback.callback(socket);
				}
			}
		}
	}
	emit(name, data) {
		for (const socket of this.sockets) {
			socket.emit(name, data);
		}
	}
	broadcast(sender, name, data) {
		for (const socket of this.sockets) {
			if (socket !== sender) {
				socket.emit(name, data);
			}
		}
	}
	delete() {
		CHANNELS.delete(this.name);
	}
	id(socket) {
		for (let i = 0; i < this.sockets.length; i++) {
			if (this.sockets[i] === socket) {
				return i;
			}
		}
		return -1;
	}
}

class Socket {
	client;
	#callbacks;

	constructor(client) {
		this.client = client;
		this.#callbacks = [];
	}
	get id() {
		return SOCKETS.indexOf(this);
	}
	on(name, callback) {
		this.#callbacks.push({ name, callback });
	}
	emit(name, data) {
		this.client.send(new ServerResponse(name, data).toString());
	}
	broadcast(name, data) {
		for (const socket of SOCKETS) {
			if (socket !== this) {
				socket.emit(name, data);
			}
		}
	}
	handleMessage(response) {
		if (typeof response !== string) {
			return;
		}

		const data = response.toString("utf-8");

		if (data === "ping") {
			return this.client.send("pong");
		} else if (data === "heartbeat") {
			return this.client.send("heartbeat");
		}

		try {
			const json = JSON.parse(data);
			for (const callback of this.#callbacks) {
				if (callback.name === json.name) {
					callback.callback(json.data);
				}
			}
		} catch (error) {
			console.error("Failed to parse WebSocket message:");
			console.error(error);
			console.info("Content:");
			console.info(response);
		}
	}
	handleClose() {
		for (const callback of this.#callbacks) {
			if (callback.name === "disconnect") {
				callback.callback(this);
			}
		}

		for (const [name, channel] of CHANNELS) {
			channel.remove(this);
		}
	}
}

export default function init(app) {
	app.ws("/socket", {
		open(client) {
			const socket = new Socket(client);
			SOCKETS.push(socket);
			for (const callback of CALLBACKS) {
				if (callback.name === "connection" ||
						callback.name === "connect") {
					callback.callback(socket);
				}
			}
		},
		message(client, message) {
			const socket = SOCKETS.find((socket) => socket.client.raw === client.raw);
			if (socket) {
				socket.handleMessage(message);
			}
		},
		close(client/*, code, message*/) {
			const socket = SOCKETS.find((socket) => socket.client.raw === client.raw);
			if (socket) {
				SOCKETS.splice(SOCKETS.indexOf(socket), 1);
				socket.handleClose();
				for (const callback of CALLBACKS) {
					if (callback.name === "disconnect") {
						callback.callback(socket);
					}
				}
			}
			client.close();
		}
	});
	app.decorate("channels", CHANNELS);
	app.decorate("sockets", SOCKETS);
	return {
		channels: CHANNELS,
		sockets: SOCKETS,
		on: function(name, callback) {
			CALLBACKS.push({ name, callback });
		},
		emit: function(name, data) {
			for (const socket of SOCKETS) {
				socket.emit(name, data);
			}
		},
		Channel,
		channel: function(name) {
			return CHANNELS[name] || null;
		}
	};
};
