import { config } from "dotenv"
config();

const PORT = process.env.PORT || 3000;

import { Elysia } from "elysia";
const app = new Elysia();

import initWebSockets from "./websockets.mjs";
const ws = initWebSockets(app);

import { staticPlugin } from "@elysiajs/static";
app.use(staticPlugin({
	assets: "public",
	prefix: "/"
}));

app
	.get("/", () => Bun.file("./public/home.html"))
	.get("/home", () => Bun.file("./public/home.html"))
	.get("/Home", () => Bun.file("./public/home.html"));

app
	.get("/projects", () => Bun.file("./public/projects.html"))
	.get("/Projects", () => Bun.file("./public/projects.html"));

app
	.get("/about", () => Bun.file("./public/about.html"))
	.get("/About", () => Bun.file("./public/about.html"));

app
	.get("/contact", () => Bun.file("./public/contact.html"))
	.get("/Contact", () => Bun.file("./public/contact.html"));

app.use(() => Bun.file("./public/404.html"));

ws.on("connect", (socket) => {
	socket.broadcast("count", ws.sockets.length);

	socket.on("count", () => {
		socket.emit("count", ws.sockets.length);
	});

	socket.on("disconnect", () => {
		socket.broadcast("count", ws.sockets.length);
	});
});

app.listen(PORT);
