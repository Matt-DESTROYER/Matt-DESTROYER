import anchor from "./anchor.js";

const nav = await fetch("../nav.json").then(function(res) {
	return res.json();
});

const header = function() {
	const head = document.createElement("header");
	const a = document.createElement("a");
	a.setAttribute("href", "./Home");
	const logo = document.createElement("img");
	logo.setAttribute("src", "../Images/logo.png");
	logo.setAttribute("height", 100);
	a.append(logo);
	const title = document.createElement("h1");
	title.textContent = "MattDESTROYER";
	a.append(title);
	head.append(a);
	const navbar = document.createElement("nav");
	for (const page of nav.pages) {
		navbar.append(anchor(page.name, page.url));
	}
	head.append(navbar);
	return head;
};

export { header as default };