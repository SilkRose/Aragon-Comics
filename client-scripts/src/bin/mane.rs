use aragon_comics_client_scripts::Worker;
use gloo_worker::Spawnable as _;
use js_sys::{JsString, Object, PropertyDescriptor};
use std::mem;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlDialogElement, window};

fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	// let worker = Worker::spawner().spawn("/worker.js");

	let window = window().unwrap();

	let descriptor = PropertyDescriptor::new();
	let closure = Closure::<dyn Fn(JsValue)>::new(open_dialog).into_js_value();
	descriptor.set_value(&closure);
	mem::forget(closure);

	Object::define_property_str(
		&window,
		&JsString::from("openDialog"),
		&descriptor
	).unwrap();

	let descriptor = PropertyDescriptor::new();
	let closure = Closure::<dyn Fn(JsValue)>::new(close_dialog).into_js_value();
	descriptor.set_value(&closure);
	mem::forget(closure);

	Object::define_property_str(
		&window,
		&JsString::from("closeDialog"),
		&descriptor
	).unwrap();
}

fn open_dialog(id: JsValue) {
	get_dialog(&id.as_string().unwrap()).show_modal().unwrap();
}

fn close_dialog(id: JsValue) {
	get_dialog(&id.as_string().unwrap()).close();
}

fn get_dialog(id: &str) -> HtmlDialogElement {
	let dialog = window()
		.unwrap()
		.document()
		.unwrap()
		.get_element_by_id(id)
		.unwrap();
	let dialog = JsValue::from(dialog);

	HtmlDialogElement::from(dialog)
}
