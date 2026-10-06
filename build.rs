use pony::command::execute_command;
use pony::fs::find_files_in_dir;
use std::env;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
	println!("cargo::rerun-if-changed=migrations");
	println!("cargo::rerun-if-changed=client-scripts");
	let mut trunk_cmd = vec!["trunk", "build"];
	if !cfg!(debug_assertions) {
		trunk_cmd.push("--release");
	}
	env::set_current_dir(Path::new("./client-scripts"))?;
	execute_command(&trunk_cmd.join(" "))?;
	env::set_current_dir(Path::new("../target/client-scripts"))?;
	let files = find_files_in_dir("./", false)?;
	let mane_js = files
		.iter()
		.find(|file| file.starts_with("./mane") && file.ends_with(".js"))
		.unwrap_or_else(|| panic!("Failed to find JS file!"));
	let mane_wasm = files
		.iter()
		.find(|file| file.starts_with("./mane") && file.ends_with(".wasm"))
		.unwrap_or_else(|| panic!("Failed to find WASM file!"));
	println!("cargo::rustc-env=MANE_JS_PATH={mane_js}");
	println!("cargo::rustc-env=MANE_WASM_PATH={mane_wasm}");
	Ok(())
}
