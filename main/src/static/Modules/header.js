import anchor from "./anchor.js";

const static_api = "https://static.matthewjames.xyz";

const nav = await fetch(static_api + "/nav.json").then(function(res) {
	return res.json();
});

const header = function() {
	const head = document.createElement("header");

	const a = document.createElement("a");
	a.setAttribute("href", "./");

	const title = document.createElement("h1");
	title.textContent = "Matthew James";
	a.append(title);

	head.append(a);

	const note = document.createElement("span");
	note.textContent = "Yeah I really didn't know what to put here...";
	head.append(note);

	const navbar = document.createElement("nav");
	for (const page of nav.pages) {
		navbar.append(anchor(page.name, page.url));
	}
	head.append(navbar);

	return head;
};

export { header as default };

