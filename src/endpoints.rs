use crate::auth::{MaybeSessionInfo, SessionInfo};
use crate::database::*;
use crate::html_templates::*;
use crate::structs::*;
use crate::utility::*;
use crate::{FimficCfg, HttpClient};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpRequest, HttpResponse, Responder, get, post};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use std::collections::{HashMap, HashSet};
use std::fs;

#[get("/style.css")]
pub async fn get_css() -> actix_web::Result<impl Responder> {
	if let Ok(ref css_file) = fs::read_to_string("./src/style.css")
		&& let Ok(mut stylesheet) = StyleSheet::parse(css_file, ParserOptions::default())
	{
		if stylesheet.minify(MinifyOptions::default()).is_err() {
			return Ok(HttpResponse::Ok()
				.content_type("text/css; charset=utf-8")
				.body(css_file.clone()));
		}
		let opts = PrinterOptions {
			minify: true,
			..Default::default()
		};
		if let Ok(styles) = stylesheet.to_css(opts) {
			Ok(HttpResponse::Ok()
				.content_type("text/css; charset=utf-8")
				.body(styles.code))
		} else {
			Ok(HttpResponse::Ok()
				.content_type("text/css; charset=utf-8")
				.body(css_file.clone()))
		}
	} else {
		Ok(HttpResponse::InternalServerError().finish())
	}
}

#[get("/mane.js")]
pub async fn get_js() -> actix_web::Result<impl Responder> {
	Ok(HttpResponse::Ok()
		.content_type("text/javascript; charset=utf-8")
		.body(fs::read_to_string("./src/mane.js")?))
}

#[get("/user")]
pub async fn get_user(
	mut db: ThinData<Db>, session: SessionInfo,
) -> actix_web::Result<impl Responder> {
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
) -> actix_web::Result<impl Responder> {
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
) -> actix_web::Result<impl Responder> {
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
) -> actix_web::Result<impl Responder> {
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
) -> actix_web::Result<impl Responder> {
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
pub async fn get_home(
	mut db: ThinData<Db>, session: MaybeSessionInfo,
) -> actix_web::Result<impl Responder> {
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
async fn oembed(query: Query<OEmbed>) -> actix_web::Result<impl Responder> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
}
