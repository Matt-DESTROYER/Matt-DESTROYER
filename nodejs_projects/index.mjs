import { config } from "dotenv"
config();

const PORT = process.env.PORT || 3000;

import { createServer } from "node:http";
import express from "express";
const app = express();
const server = createServer(app);

// get __dirname
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

app.use(express.static(join(__dirname, "static")));

app.get("/", function(req, res) {
	res
		.status(200)
		.sendFile(join(__dirname, "/static/projects.html"));
});

app.use(function(req, res) {
	res.status(404);
	res.sendFile(join(__dirname, "../public/404.html"));
});

server.listen(PORT);

