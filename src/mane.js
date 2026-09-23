function openDialog(id) {
	let dialog = window.document.getElementById(id);
	dialog.showModal();
}

function closeDialog(id) {
	let dialog = window.document.getElementById(id);
	dialog.close();
}
