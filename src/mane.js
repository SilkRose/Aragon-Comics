function openDialog(id) {
	window.document.getElementById(id).showModal();
}

function closeDialog(id) {
	window.document.getElementById(id).close();
}

function selectFiles() {
	document.getElementById('pony-up').click();
}

function previewFiles() {
	const input = window.document.getElementById("pony-up");
	const files = input.files;
	if (files.length == 0) {
		return;
	}
	const preview = document.getElementById("preview");
	for (const file of files) {
		const span = document.createElement("div");
		span.classList.add("panel-preview");
		const panel = document.createElement("img");
		panel.setAttribute("loading", "lazy");
		panel.src = URL.createObjectURL(file);
		span.appendChild(panel);
		const button = document.createElement("button");
		button.textContent = "Remove";

		span.appendChild(document.createElement("br"));
		span.appendChild(button);
		preview.appendChild(span);
	}
}

function removeFile(name) {
	const input = window.document.getElementById("pony-up");
	const panel = document.getElementById(`preview-${name}`);
}

function clearFiles() {
	document.getElementById('pony-up').files = null;
	const preview = document.getElementById('preview');
	if (preview) {
		preview.innerHTML = "";
	}
}

function uploadFiles() {
	const input = window.document.getElementById("pony-up");
	const files = input.files;
	if (files.length == 0) {
		return;
	}
	for (const file of files) {
		if (!file.type.startsWith("image/")) {
			continue;
		}
		const img = document.createElement("img");
		// img.classList.add("obj");
		img.file = file;
		window.document.appendChild(img);

		const reader = new FileReader();
		reader.onload = (e) => {
			img.src = e.target.result;
		};
		reader.readAsDataURL(file);
	}
}
