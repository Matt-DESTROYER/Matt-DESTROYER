const anchor = function(text, url) {
	const a = document.createElement("a");
	a.setAttribute("href", url);
	a.textContent = text;
	return a;
};

export { anchor as default };