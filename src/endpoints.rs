use crate::auth::{MaybeSessionInfo, SessionInfo};
use crate::database::*;
use crate::error::Result;
use crate::html_templates::*;
use crate::structs::*;
use crate::utility::*;
use crate::{FimficCfg, HttpClient};
use actix_web::web::{Data, Path, Payload, Query, ThinData};
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post};
use aws_sdk_s3::Client;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxipng::{Options, optimize_from_memory};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use tokio::fs;
use tokio::sync::Mutex;

#[cfg(not(debug_assertions))]
use actix_web::web::Bytes;
#[cfg(not(debug_assertions))]
use tokio::sync::OnceCell;

#[cfg(not(debug_assertions))]
static CSS_FILE: OnceCell<Bytes> = OnceCell::const_new();

#[cfg(not(debug_assertions))]
#[get("/style.css")]
pub async fn get_css() -> Result<impl Responder> {
	let css = CSS_FILE
		.get_or_init(|| async { Bytes::from(parse_css().await.unwrap_or_default()) })
		.await;
	match &css.is_empty() {
		true => Ok(HttpResponse::InternalServerError().finish()),
		false => Ok(HttpResponse::Ok()
			.content_type("text/css; charset=utf-8")
			.body(css.to_owned())),
	}
}

#[cfg(debug_assertions)]
#[get("/style.css")]
pub async fn get_css() -> Result<impl Responder> {
	match parse_css().await {
		Err(_) => Ok(HttpResponse::InternalServerError().finish()),
		Ok(css) => Ok(HttpResponse::Ok()
			.content_type("text/css; charset=utf-8")
			.body(css)),
	}
}

pub async fn parse_css() -> Result<String> {
	let Ok(css_file) = fs::read_to_string("./src/style.css").await else {
		// print reading error here
		return Err("Failed to read CSS file!".into());
	};
	if let Ok(mut stylesheet) = StyleSheet::parse(&css_file.clone(), ParserOptions::default())
		&& stylesheet.minify(MinifyOptions::default()).is_ok()
		&& let Ok(styles) = stylesheet.to_css(PrinterOptions {
			minify: true,
			..Default::default()
		}) {
		Ok(styles.code)
	} else {
		// print parsing error here
		Ok(css_file)
	}
}

#[cfg(not(debug_assertions))]
static JS_FILE: OnceCell<Bytes> = OnceCell::const_new();

#[cfg(not(debug_assertions))]
#[get("/mane.js")]
pub async fn get_js() -> Result<impl Responder> {
	let js = JS_FILE
		.get_or_init(|| async { Bytes::from(parse_js().await.unwrap_or_default()) })
		.await;
	match &js.is_empty() {
		true => Ok(HttpResponse::InternalServerError().finish()),
		false => Ok(HttpResponse::Ok()
			.content_type("text/css; charset=utf-8")
			.body(js.to_owned())),
	}
}

#[cfg(debug_assertions)]
#[get("/mane.js")]
pub async fn get_js() -> Result<impl Responder> {
	match parse_js().await {
		Err(_) => Ok(HttpResponse::InternalServerError().finish()),
		Ok(js) => Ok(HttpResponse::Ok()
			.content_type("text/javascript; charset=utf-8")
			.body(js)),
	}
}

pub async fn parse_js() -> Result<String> {
	let Ok(js_file) = fs::read_to_string("./src/mane.js").await else {
		// print reading error here
		return Err("Failed to read javascript file!".into());
	};
	let allocator = Allocator::default();
	let parsed = Parser::new(&allocator, &js_file, SourceType::script()).parse();
	for error in parsed.diagnostics {
		eprintln!("{error}");
	}
	if parsed.fatal_error {
		return Err("Failed to parse javascript file!".into());
	}
	let mut program = parsed.program;
	let result = Minifier::new(MinifierOptions::default()).minify(&allocator, &mut program);
	let output = Codegen::new()
		.with_options(CodegenOptions {
			minify: true,
			..CodegenOptions::default()
		})
		.with_scoping(result.scoping)
		.with_private_member_mappings(result.class_private_mappings)
		.build(&program);
	Ok(output.code)
}

#[get("/user")]
pub async fn get_user(mut db: ThinData<Db>, session: SessionInfo) -> Result<impl Responder> {
	let users = db.get_all_users().await?;
	let mut sessions = db.get_all_user_sessions(session.user_id).await?;
	sessions.sort_by_key(|k| k.last_seen);
	sessions.reverse();
	let page = user_settings_html(users, sessions);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}

#[get("/user/update/{id}")]
pub async fn set_update_user(
	req: HttpRequest, mut db: ThinData<Db>, _: SessionInfo, http_client: ThinData<HttpClient>,
	fimfic_cfg: ThinData<FimficCfg>, path: Path<i32>,
) -> Result<impl Responder> {
	let user_id = path.into_inner();
	if let Ok(user) = db.get_user(user_id).await {
		let user_update = http_client
			.get_fimfic_user(user.id, &fimfic_cfg.bearer_token)
			.await?;
		db.insert_user(user.id, &user_update.data).await?;
		Ok(HttpResponse::SeeOther()
			.append_header(("Location", redirect(req)))
			.finish())
	} else {
		let msg = "Unable to update a user who doesn't exist.";
		Ok(HttpResponse::BadRequest().body(msg))
	}
}

#[get("/user/add")]
pub async fn set_add_user(
	req: HttpRequest, mut db: ThinData<Db>, _: SessionInfo, http_client: ThinData<HttpClient>,
	fimfic_cfg: ThinData<FimficCfg>, queries: Query<HashMap<String, i32>>,
) -> Result<impl Responder> {
	let Some(user_id) = queries.into_inner().get("id").cloned() else {
		let msg = "Missing id parameter.";
		return Ok(HttpResponse::BadRequest().body(msg));
	};
	let user = db.get_user_opt(user_id).await?;
	if user.is_some() {
		let msg = "Unable to add a user who already exists.";
		return Ok(HttpResponse::BadRequest().body(msg));
	}
	let user_update = http_client
		.get_fimfic_user(user_id, &fimfic_cfg.bearer_token)
		.await?;
	db.insert_user(user_id, &user_update.data).await?;
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", redirect(req)))
		.finish())
}

#[get("/user/remove/{id}")]
pub async fn set_delete_user(
	req: HttpRequest, mut db: ThinData<Db>, session: SessionInfo, path: Path<i32>,
) -> Result<impl Responder> {
	let user_id = path.into_inner();
	if let Ok(user) = db.get_user(user_id).await {
		db.delete_user(user.id).await?;
		let location = match user.id == session.user_id {
			false => &redirect(req),
			true => "/logout",
		};
		Ok(HttpResponse::SeeOther()
			.append_header(("Location", location))
			.finish())
	} else {
		let msg = "Unable to update a user who doesn't exist.";
		Ok(HttpResponse::BadRequest().body(msg))
	}
}

#[post("/user/revoke-sessions")]
pub async fn set_revoke_sessions(
	req: HttpRequest, body: String, mut db: ThinData<Db>, session: SessionInfo,
) -> Result<impl Responder> {
	let sessions: HashSet<String> = serde_urlencoded::from_str::<HashMap<u32, String>>(&body)?
		.into_values()
		.collect();
	for session_del in &sessions {
		let check = db.get_session_by_token(session_del).await?;
		if let Some(check) = check
			&& check.user_id == session.user_id
		{
			db.delete_session(session_del).await?;
		}
	}
	let url = match sessions.contains(&session.token) {
		true => "/logout",
		false => &redirect(req),
	};
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", url))
		.finish())
}

#[get("/")]
pub async fn get_home(mut db: ThinData<Db>, session: MaybeSessionInfo) -> Result<impl Responder> {
	let user = match session.session_info {
		Some(user) => Some(db.get_user(user.user_id).await?),
		None => None,
	};
	let page = home_html(user);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}

#[get("/oembed")]
async fn oembed(query: Query<OEmbed>) -> Result<impl Responder> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
}

#[post("/comics/new")]
pub async fn set_new_comic(
	body: String, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
	let Some(title) = serde_urlencoded::from_str::<HashMap<String, String>>(&body)?
		.get("title")
		.cloned()
	else {
		let msg = "Missing title parameter.";
		return Ok(HttpResponse::BadRequest().body(msg));
	};
	let url_stub = title
		.chars()
		.filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
		.map(|c| c.to_ascii_lowercase())
		.map(|c| if c == ' ' { '-' } else { c })
		.collect::<String>();
	db.insert_comic(&title, &url_stub).await?;
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", format!("/comics/manage/{url_stub}")))
		.finish())
}

#[get("/comics")]
pub async fn get_comics(mut db: ThinData<Db>, _: SessionInfo) -> Result<impl Responder> {
	let comics = db.get_all_comics().await?;
	let mut comic_data = Vec::with_capacity(comics.len());
	for comic in comics {
		let panels = db.get_all_panels_by_comic(comic.id).await?;
		let panel_count = panels.len();
		let bytes_original = panels
			.iter()
			.fold(0, |acc, panel| acc + panel.bytes_original);
		let bytes_compressed = panels
			.iter()
			.fold(0, |acc, panel| acc + panel.bytes_compressed);
		let latest_date = panels
			.iter()
			.max_by_key(|panel| panel.date_modified)
			.map(|panel| panel.date_modified);
		let panel = ComicPanelData {
			panel_count,
			bytes_original,
			bytes_compressed,
			latest_date,
		};
		comic_data.push((comic, panel));
	}
	let data = db.get_comic_data().await?;
	let page = comic_html(comic_data, data);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}

#[get("/comics/rename/{id}")]
pub async fn set_rename_comic(
	path: Path<i32>, queries: Query<HashMap<String, String>>, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
	let id = path.into_inner();
	let Ok(_) = db.get_comic_by_id(id).await else {
		return Ok(HttpResponse::NotFound().finish());
	};
	let Some(title) = queries.into_inner().get("title").cloned() else {
		let msg = "Missing title parameter.";
		return Ok(HttpResponse::BadRequest().body(msg));
	};
	let url_stub = title
		.chars()
		.filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
		.map(|c| c.to_ascii_lowercase())
		.map(|c| if c == ' ' { '-' } else { c })
		.collect::<String>();
	db.update_comic_title(id, &url_stub, &title).await?;
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", format!("/comics/manage/{url_stub}")))
		.finish())
}

#[get("/comics/manage/{id}")]
pub async fn get_manage_comic(
	path: Path<String>, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
	let url_stub = path.into_inner();
	let comic = db.get_comic_by_url(&url_stub).await?;
	let panels = db.get_all_panels_by_comic(comic.id).await?;
	let page = manage_comic_html(comic, panels);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}

#[post("/comics/manage/{comic_id}/panel/{filename}")]
pub async fn set_comic_panel(
	path: Path<(i32, String)>, body: Payload, mut db: ThinData<Db>, s3client: ThinData<Client>,
	lock: Data<Mutex<()>>, _: SessionInfo,
) -> Result<impl Responder> {
	const MAX_SIZE: usize = 20 * 1024 * 1024; // 20MB
	let (comic_id, filename) = path.into_inner();
	let comic = db.get_comic_by_id(comic_id).await?;
	let number = Decimal::from_str(filename.to_ascii_lowercase().trim_end_matches(".png"))?;
	let Ok(data) = body.to_bytes_limited(MAX_SIZE).await else {
		return Ok(HttpResponse::PayloadTooLarge().finish());
	};
	let Ok(data) = data else {
		return Ok(HttpResponse::InternalServerError().finish());
	};
	let original_bytes = data.len() as i32;
	let mut opts = Options::from_preset(4);
	opts.fix_errors = true;
	opts.strip = oxipng::StripChunks::Safe;
	let data = {
		tokio::task::spawn_blocking(move || {
			let _guard = lock.blocking_lock();
			optimize_from_memory(&data, &opts)
		})
		.await
		.map_err(|e| format!("Compression task failed: {e}"))?
		.map_err(|e| format!("Failed to compress image: {e}"))?
	};
	let compressed_bytes = data.len() as i32;
	let (tx, panel) = db
		.insert_panel_tx(comic.id, number, original_bytes, compressed_bytes)
		.await?;
	if panel.revision > 1 {
		s3client
			.delete_object()
			.bucket("pony-r2")
			.key(panel.previous_key())
			.send()
			.await
			.map_err(|e| format!("Failed to delete image: {e}"))?;
	}
	s3client
		.put_object()
		.bucket("pony-r2")
		.content_type(String::from("image/png"))
		.key(panel.key())
		.content_length(compressed_bytes as i64)
		.body(data.into())
		.send()
		.await
		.map_err(|e| format!("Failed to upload image: {e}"))?;
	tx.commit().await?;
	Ok(HttpResponse::Ok().finish())
}

#[delete("/comics/manage/{comic_id}/panel/{filename}")]
pub async fn set_delete_panel(
	path: Path<(i32, String)>, mut db: ThinData<Db>, s3client: ThinData<Client>, _: SessionInfo,
) -> Result<impl Responder> {
	let (comic_id, number) = path.into_inner();
	let Ok(_) = db.get_comic_by_id(comic_id).await else {
		return Ok(HttpResponse::NotFound().finish());
	};
	let number = Decimal::from_str(&number)?;
	let panel = db.get_panel(comic_id, number).await?;
	s3client
		.delete_object()
		.bucket("pony-r2")
		.key(panel.key())
		.send()
		.await
		.map_err(|e| format!("Failed to delete image: {e}"))?;
	db.delete_panel(comic_id, number).await?;
	Ok(HttpResponse::Ok().finish())
}

#[get("/comics/manage/{id}/bbcode")]
pub async fn get_comic_bbcode(
	path: Path<i32>, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
	let id = path.into_inner();
	let comic = db.get_comic_by_id(id).await?;
	let panels = db.get_all_panels_by_comic(comic.id).await?;
	let mut bbcode = String::from("[center]\n");
	for panel in panels {
		bbcode.push_str("[img]");
		bbcode.push_str(&panel.link());
		bbcode.push_str("[/img]\n");
	}
	bbcode.push_str("[/center]");
	Ok(HttpResponse::Ok()
		.content_type("text/plain; charset=utf-8")
		.body(bbcode))
}

#[get("/comics/manage/{comic_id}/delete")]
pub async fn set_delete_comic(
	path: Path<i32>, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
	let comic_id = path.into_inner();
	let Ok(_) = db.get_comic_by_id(comic_id).await else {
		return Ok(HttpResponse::NotFound().finish());
	};
	let panels = db.get_all_panels_by_comic(comic_id).await?;
	if !panels.is_empty() {
		return Ok(HttpResponse::BadRequest().finish());
	}
	db.delete_comic(comic_id).await?;
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", "/comics"))
		.finish())
}

#[get("/comics/{title}")]
pub async fn get_comic(
	path: Path<String>, mut db: ThinData<Db>, session: MaybeSessionInfo,
) -> Result<impl Responder> {
	let title = path.into_inner();
	let comic = db.get_comic_by_url_count_hit(&title).await?;
	let panels = db.get_all_panels_by_comic(comic.id).await?;
	let page = read_comic_html(comic, panels, session.session_info.is_some());
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}
