const { Server } = require("ws");
const wss = new Server({ noServer: true });

const CHANNELS = {};
const CLIENTS = [];
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
		CHANNELS[this.name] = this;
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
				if (callback.name === "connect") {
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
		delete CHANNELS[this.name];
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
	#client;
	#callbacks;

	constructor(client) {
		this.#client = client;
		this.#callbacks = [];
		this.#client.on("message", function(data) {
			const _data = data.toString("utf-8");
			if (_data === "ping") {
				return this.#client.send("pong");
			} else if (_data === "heartbeat") {
				return this.#client.send("heartbeat");
			}
			const { name, data: __data } = JSON.parse(_data);
			for (const callback of this.#callbacks) {
				if (callback.name === name) {
					callback.callback(__data);
				}
			}
		}.bind(this));
		this.#client.on("close", function() {
			CLIENTS.splice(CLIENTS.indexOf(this.#client), 1);
			SOCKETS.splice(SOCKETS.indexOf(this), 1);
			for (const callback of this.#callbacks) {
				if (callback.name === "disconnect") {
					callback.callback(this);
				}
			}
			for (const callback of CALLBACKS) {
				if (callback.name === "disconnect") {
					callback.callback(this);
				}
			}
			for (const channel of Object.keys(CHANNELS)) {
				CHANNELS[channel].remove(this);
			}
		}.bind(this));
	}
	get id() {
		return SOCKETS.indexOf(this);
	}
	on(name, callback) {
		this.#callbacks.push({ name, callback });
	}
	send(name, data) {
		this.#client.send(new ServerResponse(name, data).toString());
	}
	emit(name, data) {
		for (const socket of SOCKETS) {
			socket.send(name, data);
		}
	}
	broadcast(name, data) {
		for (const socket of SOCKETS) {
			if (socket !== this) {
				socket.send(name, data);
			}
		}
	}
}

module.exports = function init(server) {
	server.on("upgrade", function(req, _socket, head) {
		if (req.url === "/socket") {
			wss.handleUpgrade(req, _socket, head, function(client) {
				CLIENTS.push(client);
				const socket = new Socket(client);
				SOCKETS.push(socket);
				for (const callback of CALLBACKS) {
					if (callback.name === "connection" ||
							callback.name === "connect") {
						callback.callback(socket, req);
					}
				}
			});
		} else {
			_socket.destroy();
		}
	});
	return {
		SOCKETS,
		CHANNELS,
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
