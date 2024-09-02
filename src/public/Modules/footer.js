const footer = function(socket) {
	const foot = document.createElement("footer");
	const info = document.createElement("div");
	info.setAttribute("style", "display: flex;");
	const info_image = document.createElement("img");
	info_image.classList.add("language-icon");
	info_image.setAttribute("title", "NodeJS");
	info_image.setAttribute("src", "../Images/NodeJS.png");
	info.append(info_image);
	const info_text = document.createElement("p");
	info_text.setAttribute("style", "margin-left: 5px;");
	info_text.innerHTML = "This website was created using <a href=\"https://nodejs.org/\">NodeJS</a> and is hosted on <a href=\"https://replit.com/\">Replit<a>.";
	info.append(info_text);
	foot.append(info);
	const copyright = document.createElement("p");
	copyright.textContent = "This site is Matthew James' portfolio site. To contact Matthew, refer to the contact page.";
	foot.append(copyright);
	return foot;
};

export { footer as default };