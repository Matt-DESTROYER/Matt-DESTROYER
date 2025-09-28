const footer = function(socket) {
	const foot = document.createElement("footer");

	const info = document.createElement("div");
	info.setAttribute("style", "display: flex;");

	const info_image = document.createElement("img");
	info_image.classList.add("language-icon");
	info_image.setAttribute("title", "Rust");
	info_image.setAttribute("src", "../Images/Rust.jpeg");
	info.append(info_image);

	const info_text = document.createElement("p");
	info_text.setAttribute("style", "margin-left: 5px;");
	info_text.innerHTML = "This website was created using <a href=\"https://rust-lang.org\">Rust</a> and is self-hosted using a home-built personal Ubuntu server and a Cloudflare tunnel!";
	info.append(info_text);

	foot.append(info);

	const copyright = document.createElement("p");
	copyright.innerHTML = "© Matthew James 2025. To contact Matthew, please refer to the <a href=\"./Contact\">contact page</a>.";

	foot.append(copyright);

	return foot;
};

export { footer as default };
