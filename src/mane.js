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
	const files = Array.from(window.document.getElementById("pony-up").files);
	if (files.length == 0) {
		return;
	}
	files.sort((a, b) =>
		Number(a.name.replace(".png", "")) -
		Number(b.name.replace(".png", ""))
	);
	const preview = document.getElementById("preview-panels");
	for (const file of files) {
		const div = document.createElement("div");
		div.classList.add("panel-div");
		div.id = `preview-${file.name}`;
		const panel = document.createElement("img");
		panel.setAttribute("loading", "lazy");
		panel.src = URL.createObjectURL(file);
		div.appendChild(panel);
		const sub_div = document.createElement("div");
		sub_div.classList.add("panel-info");
		const p = document.createElement("p");
		p.innerHTML = `<b>Filename:</b> ${file.name}`;
		sub_div.appendChild(p);
		const button = document.createElement("button");
		button.classList.add("danger");
		button.textContent = "Remove";
		button.onclick = () => removeFile(file.name);
		sub_div.appendChild(button);
		div.appendChild(sub_div);
		preview.appendChild(div);
	}
}

function removeFile(name) {
	const input = document.getElementById("pony-up");
	const panel = document.getElementById(`preview-${name}`);
	const new_files = new DataTransfer();
	for (const file of input.files) {
		if (file.name !== name) {
			new_files.items.add(file);
		}
	}
	if (panel != null) {
		panel.remove();
	}
	input.files = new_files.files;
}

function clearFiles() {
	document.getElementById('pony-up').files = null;
	const preview = document.getElementById('preview-panels');
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
