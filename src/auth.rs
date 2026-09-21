use crate::database::*;
use crate::error::{Error, Result};
use crate::fimfic_cfg::FimficCfg;
use crate::http::{FimficTokenExchangeResponse, HttpClient};
use crate::rand::{gen_auth_state, gen_auth_token};
use crate::structs::User;
use actix_web::dev::Payload;
use actix_web::http::header::{HeaderValue, USER_AGENT};
use actix_web::web::{Path, Query, ThinData as Data};
use actix_web::{FromRequest, get};
use actix_web::{HttpRequest, HttpResponse};
use anyhow::Context as _;
use bon::builder;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Deserialize)]
struct AuthQueryParams {
	code: Option<String>,
	state: Option<String>,
}

#[get("/login/fimfic")]
pub async fn fimfic_auth(
	req: HttpRequest, Query(form): Query<AuthQueryParams>, db: Data<Db>,
	fimfic_cfg: Data<FimficCfg>, http_client: Data<HttpClient>,
) -> HttpResponse {
	if let Some(code) = form.code
		&& let Some(state) = form.state
	{
		fimfic_auth_return()
			.code(code)
			.state(state)
			.req(req)
			.db(db)
			.fimfic_cfg(fimfic_cfg)
			.http_client(http_client)
			.call()
			.await
	} else {
		fimfic_auth_redirect(req, db, fimfic_cfg).await
	}
}

#[get("/logout")]
pub async fn fimfic_auth_logout(req: HttpRequest, mut db: Data<Db>) -> HttpResponse {
	let mut response = HttpResponse::SeeOther();
	response.insert_header(("location", "/"));

	if let Some(cookie) = cookie::try_get_session_cookie(&req) {
		// try to delete it but it doesn't really matter if it doesn't work?
		let _ = db.delete_session(cookie.value()).await;

		response.cookie(cookie::create_unset_session_cookie());
	}

	if cookie::try_get_session_info_cookie(&req).is_some() {
		response.cookie(cookie::create_unset_session_info_cookie());
	}

	response.finish()
}

async fn fimfic_auth_redirect(
	req: HttpRequest, mut db: Data<Db>, fimfic_cfg: Data<FimficCfg>,
) -> HttpResponse {
	if let Some(session_cookie) = cookie::try_get_session_cookie(&req) {
		let session = db.update_session_last_seen(session_cookie.value()).await;

		match session {
			Ok(None) => {
				// invalid cookie; continue with regular auth flow
			}
			Ok(Some(_session)) => {
				return HttpResponse::Ok()
					.content_type("text/plain")
					.body("already have cookie (validated to be valid session)");
			}
			Err(err) => {
				eprintln!("wah {err}");
				// todo present actual error
				return HttpResponse::Ok()
					.content_type("text/plain")
					.body("db error trying to get existing session");
			}
		}
	}

	let state = gen_auth_state();

	let login_url = format!(
		"{login_url}&state={state}",
		login_url = &*fimfic_cfg.login_url
	);

	HttpResponse::SeeOther()
		.append_header(("location", login_url))
		.cookie(cookie::create_state_cookie(&state))
		.finish()
}

#[builder]
async fn fimfic_auth_return(
	req: HttpRequest, mut db: Data<Db>, fimfic_cfg: Data<FimficCfg>, http_client: Data<HttpClient>,
	code: String, state: String,
) -> HttpResponse {
	let Some(state_cookie) = cookie::try_get_state_cookie(&req) else {
		// todo present an actual error
		return HttpResponse::Ok()
			.content_type("text/plain")
			.body("login failed: no cookie found");
	};

	let state_cookie = state_cookie.value();
	if state_cookie != &*state {
		// todo present an actual error
		return HttpResponse::Ok().content_type("text/plain").body(format!(
			"login failed: state mismatch\n\nstate param: {state}\nstate cookie: {state_cookie}"
		));
	}

	let FimficTokenExchangeResponse {
		user_id,
		name: _,
		access_token,
	} = match http_client.fimfic_token_exchange(&fimfic_cfg, &code).await {
		Ok(res) => res,
		Err(err) => {
			eprintln!("error in fimfic token exchange: {err}");
			// todo present an actual error
			return HttpResponse::Ok()
				.content_type("text/plain")
				.body("login failed: fimfic code exchange error");
		}
	};

	let fimfic_user = match http_client.get_fimfic_user(user_id, &access_token).await {
		Ok(res) => res,
		Err(err) => {
			eprintln!("error in pfp fetching: {err}");
			// todo present an actual error
			return HttpResponse::Ok()
				.content_type("text/plain")
				.body("login failed: error fetching profile picture");
		}
	};

	let db_result = db.insert_user(user_id, &fimfic_user.data).await;

	if let Err(err) = db_result {
		eprintln!("error in db storing: {err}");
		// todo present an actual error
		return HttpResponse::Ok()
			.content_type("text/plain")
			.body("login failed: error storing user");
	}

	let token = gen_auth_token();
	let user_agent = req
		.headers()
		.get(USER_AGENT)
		.cloned()
		.unwrap_or(HeaderValue::from_static("Unknown"));
	let user_agent = user_agent.to_str().unwrap_or("Unknown");

	if let Err(err) = db.insert_session(&token, user_id, user_agent).await {
		eprintln!("error in token storing {err}");
		return HttpResponse::Ok()
			.content_type("text/plain")
			.body("storing token in db broke");
	}

	let pfp_url = fimfic_user
		.data
		.attributes
		.avatar
		.r256
		.trim_end_matches("-256");

	// todo redirect to home page or something
	HttpResponse::SeeOther()
		.cookie(cookie::create_unset_state_cookie())
		.cookie(cookie::create_session_cookie(&token))
		.cookie(cookie::create_session_info_cookie(user_id, pfp_url))
		.insert_header(("location", "/"))
		.finish()
}

/// Session info extractor
pub struct SessionInfo {
	pub user_id: i32,
	pub token: String,
}

impl FromRequest for SessionInfo {
	type Error = Error;
	type Future = impl Future<Output = Result<SessionInfo>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		let req = req.clone();

		async move {
			let session_info = get_unverified_session_info(&req).context("not logged in")?;
			verify_session_info(&req, &session_info).await?;
			Ok(session_info)
		}
	}
}

/// Optional session info extractor (user can be logged in or not)
pub struct MaybeSessionInfo {
	pub session_info: Option<SessionInfo>,
}

impl FromRequest for MaybeSessionInfo {
	type Error = Error;
	type Future = impl Future<Output = Result<MaybeSessionInfo>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		// req has inner Rc so it's cheap to clone
		let req = req.clone();
		let session_info = get_unverified_session_info(&req);

		async move {
			if let Some(session_info) = &session_info {
				verify_session_info(&req, session_info).await?
			}

			Ok(MaybeSessionInfo { session_info })
		}
	}
}

fn get_unverified_session_info(req: &HttpRequest) -> Option<SessionInfo> {
	let session = cookie::try_get_session_cookie(req)?;
	let session_info = cookie::try_get_session_info_cookie_value(req)?;

	Some(SessionInfo {
		user_id: session_info.user_id,
		token: session.value().into(),
	})
}

async fn verify_session_info(req: &HttpRequest, session_info: &SessionInfo) -> Result<()> {
	let mut db = get_db(req)?;

	let db_session_info = db
		.update_session_last_seen(&session_info.token)
		.await?
		.context("invalid session token")?;

	if db_session_info.user_id != session_info.user_id {
		return Err(anyhow::format_err!(
			"invalid session info, user ID does not match session token"
		)
		.into());
	}

	Ok(())
}

async fn user_from_session_info(req: &HttpRequest, session_info: &SessionInfo) -> Result<User> {
	let mut db = get_db(req)?;
	let user = db.get_user(session_info.user_id).await?;
	Ok(user)
}

fn get_db(req: &HttpRequest) -> Result<Data<Db>> {
	Ok(req
		.app_data::<Data<Db>>()
		.context("no ThinData<Db> found")?
		.clone())
}

#[derive(Clone, Deserialize)]
pub struct DevSession {
	inner: Arc<DevSessionInner>,
}

#[derive(Clone, Deserialize)]
pub struct DevSessionInner {
	token: String,
	user_id: i32,
	pfp_url: String,
}

impl DevSession {
	pub fn new(token: String, user_id: i32, pfp_url: String) -> Self {
		Self {
			inner: Arc::new(DevSessionInner {
				token,
				user_id,
				pfp_url,
			}),
		}
	}
}

#[get("/dev-session/{token}")]
pub async fn dev_session(
	req: HttpRequest, path: Path<String>, mut db: Data<Db>, dev_session: Data<Option<DevSession>>,
) -> HttpResponse {
	let Some(DevSession { inner: dev_session }) = &*dev_session else {
		return HttpResponse::NotFound().finish();
	};

	let user_agent = req
		.headers()
		.get(USER_AGENT)
		.cloned()
		.unwrap_or(HeaderValue::from_static("Unknown"));
	let user_agent = user_agent.to_str().unwrap_or("Unknown");

	if **path != *dev_session.token {
		HttpResponse::NotFound().finish()
	} else if db
		.insert_session(&dev_session.token, dev_session.user_id, user_agent)
		.await
		.is_err()
	{
		HttpResponse::InternalServerError().finish()
	} else {
		HttpResponse::Ok()
			.content_type("text/plain")
			.cookie(cookie::create_session_cookie(&dev_session.token))
			.cookie(cookie::create_session_info_cookie(
				dev_session.user_id,
				&dev_session.pfp_url,
			))
			.body("token set")
	}
}

mod cookie {
	use super::*;

	use actix_web::cookie::time::Duration;
	use actix_web::cookie::{Cookie, SameSite};

	const STATE_COOKIE_NAME: &str = "fimfic-auth-state";
	const STATE_COOKIE_PATH: &str = "/login/fimfic";

	pub fn create_state_cookie(state: &str) -> Cookie<'_> {
		Cookie::build(STATE_COOKIE_NAME, state)
			.path(STATE_COOKIE_PATH)
			.max_age(Duration::hours(1))
			.same_site(SameSite::Lax)
			.http_only(true)
			.secure(true)
			.finish()
	}

	pub fn try_get_state_cookie(req: &HttpRequest) -> Option<Cookie<'static>> {
		req.cookie(STATE_COOKIE_NAME)
	}

	pub fn create_unset_state_cookie() -> Cookie<'static> {
		Cookie::build(STATE_COOKIE_NAME, "3c")
			.path(STATE_COOKIE_PATH)
			.max_age(Duration::ZERO)
			.finish()
	}

	const SESSION_COOKIE_NAME: &str = "fimfic-auth-session";
	const SESSION_COOKIE_PATH: &str = "/";

	pub fn create_session_cookie(token: &str) -> Cookie<'_> {
		Cookie::build(SESSION_COOKIE_NAME, token)
			.path(SESSION_COOKIE_PATH)
			.max_age(Duration::days(30))
			.same_site(SameSite::Lax)
			.http_only(true)
			.secure(true)
			.finish()
	}

	pub fn try_get_session_cookie(req: &HttpRequest) -> Option<Cookie<'static>> {
		req.cookie(SESSION_COOKIE_NAME)
	}

	pub fn create_unset_session_cookie() -> Cookie<'static> {
		Cookie::build(SESSION_COOKIE_NAME, "3c")
			.path(SESSION_COOKIE_PATH)
			.max_age(Duration::ZERO)
			.finish()
	}

	#[derive(Deserialize, Serialize)]
	pub struct SessionInfo<'h> {
		pub user_id: i32,
		pub pfp_url: Cow<'h, str>,
	}

	const SESSION_INFO_COOKIE_NAME: &str = "fimfic-auth-session-info";
	const SESSION_INFO_COOKIE_PATH: &str = SESSION_COOKIE_PATH;

	pub fn create_session_info_cookie(user_id: i32, pfp_url: &str) -> Cookie<'static> {
		let value = serde_json::to_string(&SessionInfo {
			user_id,
			pfp_url: Cow::Borrowed(pfp_url),
		})
		.unwrap();

		Cookie::build(SESSION_INFO_COOKIE_NAME, value)
			.path(SESSION_INFO_COOKIE_PATH)
			.max_age(Duration::days(30))
			.same_site(SameSite::Lax)
			.http_only(false)
			.secure(true)
			.finish()
	}

	pub fn try_get_session_info_cookie(req: &HttpRequest) -> Option<Cookie<'static>> {
		req.cookie(SESSION_INFO_COOKIE_NAME)
	}

	pub fn try_get_session_info_cookie_value(req: &HttpRequest) -> Option<SessionInfo<'static>> {
		try_get_session_info_cookie(req)
			.and_then(|cookie| serde_json::from_str(cookie.value()).ok())
	}

	pub fn create_unset_session_info_cookie() -> Cookie<'static> {
		Cookie::build(SESSION_INFO_COOKIE_NAME, "3c")
			.path(SESSION_INFO_COOKIE_PATH)
			.max_age(Duration::ZERO)
			.finish()
	}
}
