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
	let input = window.document.getElementById("pony-up");
	let files = input.files;
	if (files.length == 0) {
		return;
	}
	for (const file in files) {
		console.log(file.name)
	}
}

function clearFiles() {
	document.getElementById('pony-up').files = null;
	let preview = document.getElementById('panel-preview');
	if (preview) {
		preview.remove();
	}
}

function uploadFiless() {
	let input = window.document.getElementById("pony-up");
	let files = input.files;
	if (files.length == 0) {
		return;
	}
	for (const file in files) {
		if (!file.type.startsWith("image/")) {
			continue;
		}
		const img = document.createElement("img");
		// img.classList.add("obj");
		img.file = file;
		preview.appendChild(img);

		const reader = new FileReader();
		reader.onload = (e) => {
			img.src = e.target.result;
		};
		reader.readAsDataURL(file);
	}
}
