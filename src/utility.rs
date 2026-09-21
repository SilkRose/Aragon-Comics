use actix_web::HttpRequest;

pub fn redirect(req: HttpRequest) -> String {
	req.headers()
		.get("Referer")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("/")
		.into()
}
