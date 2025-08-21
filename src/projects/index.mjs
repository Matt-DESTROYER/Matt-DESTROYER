import express from "express";
const app = express();
import { createServer } from "node:http";
const server = createServer(app);
import { Server } from "socket.io";
const io = new Server(server);

// get __dirname
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

app.use(express.static(join(__dirname, "static")));

app.get("/", function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "/public/projects.html"));
});

app.use(function(req, res) {
	res.status(404);
	res.sendFile(join(__dirname, "../public/404.html"));
});

(function initSocketio() {
	let usersConnected = 0;

	io.on("connection", function(socket) {
		usersConnected++;

		io.emit("count", usersConnected);

		socket.on("count", function() {
			io.emit("count", usersConnected);
		});

		socket.on("disconnect", function() {
			usersConnected--;
			io.emit("count", usersConnected);
		});
	});
})();

server.listen(process.env.PORT || 3000);
