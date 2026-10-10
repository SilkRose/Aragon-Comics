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
	const input = document.getElementById("pony-up");
	const files = Array.from(input.files);
	if (files.length == 0) {
		return;
	}
	files.sort((a, b) =>
		Number(a.name.toLowerCase().replace(".png", "")) -
		Number(b.name.toLowerCase().replace(".png", ""))
	);
	const sorted_files = new DataTransfer();
	for (const file of files) {
		sorted_files.items.add(file);
	}
	input.files = sorted_files.files;
	const preview = document.getElementById('preview-panels');
	if (preview) {
		preview.innerHTML = "";
	}
	for (const file of input.files) {
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
		button.onclick = () => openDialog(file.name);
		sub_div.appendChild(button);
		div.appendChild(sub_div);
		const dialog = document.createElement("dialog");
		dialog.id = file.name;
		dialog.closedBy = "any";
		const dialog_title = document.createElement("h2");
		dialog_title.innerText = "Remove Panel";
		dialog.appendChild(dialog_title);
		let dialog_panel = document.createElement("img");
		dialog_panel.setAttribute("loading", "lazy");
		dialog_panel.src = URL.createObjectURL(file);
		dialog.appendChild(dialog_panel);
		const dialog_text = document.createElement("p");
		dialog_text.innerText = "Are you sure you want to remove this panel before it's uploaded?";
		dialog.appendChild(dialog_text);
		const dialog_span = document.createElement("span");
		dialog_span.classList.add("spaced-row");
		const dialog_close_button = document.createElement("button");
		dialog_close_button.textContent = "Close";
		dialog_close_button.onclick = () => closeDialog(file.name);
		dialog_span.appendChild(dialog_close_button);
		const dialog_remove_button = document.createElement("button");
		dialog_remove_button.classList.add("danger");
		dialog_remove_button.textContent = "Remove";
		dialog_remove_button.onclick = () => removeFile(file.name);
		dialog_span.appendChild(dialog_remove_button);
		dialog.appendChild(dialog_span);
		div.appendChild(dialog);
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
	if (panel) {
		panel.remove();
	}
	input.files = new_files.files;
}

function clearSelection() {
	let count = document.getElementById('pony-up').files.length;
	if (count > 0) {
		document.getElementById("panel-count").innerText = count;
		openDialog("clear-selection");
	}
}

function clearFiles() {
	document.getElementById('pony-up').value = null;
	const preview = document.getElementById('preview-panels');
	if (preview) {
		preview.innerHTML = "";
	}
	closeDialog("clear-selection");
}

let abort_upload = false;

function cancelUpload() {
	abort_upload = true;
}

async function uploadFiles() {
	abort_upload = false;
	const upload = window.document.getElementById("panel-upload");
	const input = window.document.getElementById("pony-up");
	const files = input.files;
	const count = files.length;
	if (count == 0) {
		return;
	}
	const comic_id = upload.getAttribute("data-comic-id");
	const endpoint = `/comics/manage/${comic_id}/panel`;
	window.document.getElementById("panel-total").innerText = count;
	const panel_index = window.document.getElementById("panel-index");
	const panel_percent = window.document.getElementById("panel-percent");
	const panel_progress = document.getElementById("progress");
	panel_progress.max = count;
	const panel_preview = document.getElementById("upload-preview");
	const countdown = document.getElementById("time-left");
	upload.showModal();
	let i = 1;
	const times = [];
	let total_time = 0;
	let interval = null;
	for (const file of files) {
		const start_time = new Date();
		panel_index.innerText = i;
		panel_progress.value = i;
		if (abort_upload) {
			upload.close();
			break;
		}
		if (file.type !== "image/png") {
			alert("Invalid file in list!");
			break;
		}
		panel_preview.src = URL.createObjectURL(file);
		if (times.length) {
			const average = total_time / times.length;
			const remaining = average * (count - i);
			const target = new Date(Date.now() + remaining);
			if (!interval && i > 1 || i % 10 == 0) {
				if (interval) {
					clearInterval(interval);
				}
				interval = setInterval(function () {
					let remaining = Math.max(0, (target - new Date()) / 1000);
					let timer = calculateTime(remaining);
					countdown.innerHTML = timer;
					if (remaining <= 0) {
						clearInterval(interval);
						countdown.innerHTML = "Reloading…";
					}
				}, 1000);
			}
		}
		try {
			await uploadFile(file, endpoint);
		} catch (error) {
			alert(error.message);
		}
		await sleep(100);
		panel_percent.innerText = ((i++ / count) * 100).toFixed(2);
		const time = new Date() - start_time;
		total_time += time;
		times.push(time);
	}
	window.document.location.reload();
}

const sleep = ms => new Promise(r => setTimeout(r, ms));

function calculateTime(remaining) {
	let days = Math.floor(remaining / 86400);
	let hours = Math.floor((remaining % 86400) / 3600);
	let minutes = Math.floor((remaining % 3600) / 60);
	let secs = Math.floor(remaining % 60);
	let parts = [];
	if (days) parts.push(formatPlural(days, "day"));
	if (hours) parts.push(formatPlural(hours, "hour"));
	if (minutes) parts.push(formatPlural(minutes, "minute"));
	if (secs) parts.push(formatPlural(secs, "second"));
	if (parts.length === 0) {
		parts.push("0 seconds");
	}
	return parts.join(", ");
}

function formatPlural(value, unit) {
	return `${value} ${unit}${value !== 1 ? "s" : ""}`;
}

function uploadFile(file, endpoint) {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open("POST", `${endpoint}/${file.name}`);
		xhr.setRequestHeader("Content-Type", "image/png");
		xhr.onload = () => {
			if (xhr.status < 200 || xhr.status >= 300) {
				return reject(new Error(`Error processing file: ${file.name}`));
			}
			return resolve(xhr.responseText);
		};
		xhr.onerror = () => {
			reject(new Error(`Failed to upload file: ${file.name}`));
		};
		xhr.send(file);
	});
}
