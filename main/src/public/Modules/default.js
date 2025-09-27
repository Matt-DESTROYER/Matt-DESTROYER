import header from "./header.js";
import footer from "./footer.js";

document.body.prepend(header());
document.body.append(footer());

import Socket from "./socket.js";

(function initCounter() {
	const count_message = "People viewing this portfolio (a live counter): ";

	const display = document.createElement("p");
	display.textContent = count_message + 1;

	// create websocket connection to server to live update the count of connected users
	const socket = new Socket();
	socket.on("count", function(count) {
		display.textContent = count_message + count;
	});
	socket.on("connect", () => socket.emit("count"));

	document.getElementsByTagName("footer")[0].prepend(display);
})();