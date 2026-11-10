use actix_web::HttpRequest;
use pony::number_format::linear_units::{
	BINARY_DIVISOR_I64, METRIC_BYTE_ABBREVIATIONS, format_number_unit_i64,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn redirect(req: HttpRequest) -> String {
	req.headers()
		.get("Referer")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("/")
		.into()
}

#[cfg(debug_assertions)]
pub fn js_url() -> String {
	format!("/mane.js?{}", unix_timestamp())
}

#[cfg(not(debug_assertions))]
pub fn js_url() -> String {
	format!("/mane.js?{}", env!("JS_HASH"))
}

#[cfg(debug_assertions)]
pub fn css_url() -> String {
	format!("/style.css?{}", unix_timestamp())
}

#[cfg(not(debug_assertions))]
pub fn css_url() -> String {
	format!("/style.css?{}", env!("CSS_HASH"))
}

#[cfg(debug_assertions)]
fn unix_timestamp() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
}

pub fn format_bytes(bytes: i64) -> String {
	format_number_unit_i64()
		.number(bytes)
		.decimal_places(2)
		.divisor(BINARY_DIVISOR_I64)
		.units(&METRIC_BYTE_ABBREVIATIONS)
		.spaced(true)
		.trim_tail(true)
		.plural_ess(false)
		.call()
}
