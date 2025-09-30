import header from "./header.js";
import footer from "./footer.js";

const main_element = document.querySelector("main");

const header_element = header(), footer_element = footer();
main_element.prepend(header_element);
main_element.append(footer_element);

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

	footer_element.prepend(display);
})();
