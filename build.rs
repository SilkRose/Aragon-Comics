use sha2::{Digest, Sha256};
use std::fs;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
	println!("cargo::rerun-if-changed=migrations");
	println!("cargo::rerun-if-changed=src/mane.js");
	println!("cargo::rerun-if-changed=src/style.css");
	let js_file = fs::read("./src/mane.js")?;
	let js_hash = Sha256::digest(js_file);
	let js_hex = const_hex::encode(js_hash);
	println!("cargo::rustc-env=JS_HASH={}", &js_hex[..8]);
	let css_file = fs::read("./src/style.css")?;
	let css_hash = Sha256::digest(css_file);
	let css_hex = const_hex::encode(css_hash);
	println!("cargo::rustc-env=CSS_HASH={}", &css_hex[..8]);
	Ok(())
}
