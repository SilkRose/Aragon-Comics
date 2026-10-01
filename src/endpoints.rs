use crate::auth::{MaybeSessionInfo, SessionInfo};
use crate::database::*;
use crate::error::Result;
use crate::html_templates::*;
use crate::structs::*;
use crate::utility::*;
use crate::{FimficCfg, HttpClient};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpRequest, HttpResponse, Responder, get, post};
use aws_sdk_s3::Client;
use aws_sdk_s3::presigning::PresigningConfigBuilder;
use chrono::Utc;
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use tokio::fs;
use uuid::Uuid;

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

#[get("/mane.js")]
pub async fn get_js() -> Result<impl Responder> {
	Ok(HttpResponse::Ok()
		.content_type("text/javascript; charset=utf-8")
		.body(fs::read_to_string("./src/mane.js").await?))
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

#[get("/comics/new")]
pub async fn set_new_comic(
	queries: Query<HashMap<String, String>>, mut db: ThinData<Db>, _: SessionInfo,
) -> Result<impl Responder> {
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
	db.insert_comic(&title, &url_stub).await?;
	Ok(HttpResponse::SeeOther()
		.append_header(("Location", format!("/comic/manage/{url_stub}")))
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
	let page = comic_html(comic_data);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(page))
}

#[get("/comics/rename/{id}")]
pub async fn set_rename_comic(
	path: Path<String>, queries: Query<HashMap<String, String>>, mut db: ThinData<Db>,
	_: SessionInfo,
) -> Result<impl Responder> {
	let id = Uuid::from_str(&path.into_inner())?;
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
	let comic = db.get_comic_by_id(id).await?;
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

#[post("/comics/manage/{id}/panel")]
pub async fn set_comic_panel(
	path: Path<String>, body: String, mut db: ThinData<Db>, s3client: ThinData<Client>,
	_: SessionInfo,
) -> Result<impl Responder> {
	let id = Uuid::from_str(&path.into_inner())?;
	let comic = db.get_comic_by_id(id).await?;
	let data = serde_json::from_str::<PanelData>(&body)?;
	let number = data.filename.to_ascii_lowercase();
	let number = number.trim_end_matches(".png");
	let number = Decimal::from_str(number)?;
	let key = format!("{}/{number}-{}", comic.id, data.hash);
	let config = PresigningConfigBuilder::default()
		.start_time(SystemTime::from(Utc::now()))
		.expires_in(Duration::from_mins(15))
		.build()?;
	let url = s3client
		.put_object()
		.bucket("pony-r2")
		.content_encoding(String::from("image/png"))
		.key(key)
		.presigned(config)
		.await?;
	db.insert_panel(
		comic.id,
		number,
		&data.hash,
		data.bytes_original,
		data.bytes_compressed,
	)
	.await?;
	Ok(HttpResponse::Ok().body(url.uri().to_string()))
}
