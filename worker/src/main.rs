use wasm_bindgen::JsValue;
use web_sys::DedicatedWorkerGlobalScope;

fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));

	let this = JsValue::from(js_sys::global());
	let this = DedicatedWorkerGlobalScope::from(this);
}
