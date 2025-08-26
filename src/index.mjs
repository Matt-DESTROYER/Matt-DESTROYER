import { config } from "dotenv"
config();

const PORT = process.env.PORT || 3000;

import { createServer } from "node:http";
import express from "express";
const app = express();
const server = createServer(app);

import initWebSockets from "./websockets.js";
const ws = initWebSockets(server);

// get __dirname
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

app.use(express.static(join(__dirname, "public")));

app.use(function(req, res, next) {
	next();
});

app.get(["/", "/home", "/Home"], function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "public/home.html"));
});
app.get(["/projects", "/Projects"], function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "public/projects.html"));
});
app.get(["/about", "/About"], function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "public/about.html"));
});
app.get(["/contact", "/Contact"], function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "public/contact.html"));
});

app.use(function(req, res) {
	res.status(404);
	res.sendFile(join(__dirname, "public/404.html"));
});

(function initWebSockets() {
	ws.on("connection", function(socket) {
		socket.broadcast("count", ws.SOCKETS.length);

		socket.on("count", function() {
			socket.emit("count", ws.SOCKETS.length);
		});

		socket.on("disconnect", function() {
			socket.broadcast("count", ws.SOCKETS.length);
		});
	});
})();

server.listen(PORT);
