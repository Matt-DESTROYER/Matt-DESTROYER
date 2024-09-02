import express from "express";
const app = express();
import { createServer } from "node:http";
const server = createServer(app);
import { Server } from "socket.io";
const io = new Server(server);

// get __dirname
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

app.use(express.static("public"));

app.get(["/", "/Home"], function(req, res) {
	res.status(200);
	res.sendFile(__dirname + "/public/home.html");
});
app.get("/Projects", function(req, res) {
	res.status(200);
	res.sendFile(__dirname + "/public/projects.html");
});
app.get("/About", function(req, res) {
	res.status(200);
	res.sendFile(__dirname + "/public/about.html");
});
app.get("/Contact", function(req, res) {
	res.status(200);
	res.sendFile(__dirname + "/public/contact.html");
});

app.get("*", function(req, res) {
	res.status(404);
	res.sendFile(__dirname + "/public/404.html");
});

(function intSocketio() {
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
