#![feature(impl_trait_in_assoc_type)]

use crate::endpoints::*;

pub use self::database::*;
pub use self::error::Result;
pub use self::fimfic_cfg::FimficCfg;
pub use self::http::HttpClient;

pub use actix_files::Files;
pub use actix_web::middleware::Compress;
pub use actix_web::web::ThinData as Data;
pub use actix_web::{App as ActixApp, HttpServer};
pub use aws_sdk_s3::config::{Credentials, Region};
pub use aws_sdk_s3::{Client, Config};

mod auth;
mod database;
mod endpoints;
mod env_vars;
mod error;
mod fimfic_cfg;
mod html_templates;
mod http;
mod rand;
mod structs;
mod utility;

pub const PORT: u16 = 4663; // hoof
pub const SITE_NAME: &str = "Aragón Comics";
pub const SITE_LINK: &str = if cfg!(debug_assertions) {
	"http://127.0.0.1:4663"
} else {
	"https://aragon-comics.silkrose.dev"
};

#[actix_web::main]
async fn main() -> Result<()> {
	env_vars::load_dotenv();
	env_vars::check();

	if cfg!(debug_assertions) {
		assert!(
			SITE_LINK.ends_with(&PORT.to_string()),
			"Port mismatched with site link in dev!"
		)
	}

	let db = Db::new(&env_vars::database_url()).await?;
	let mut db = Data(db);

	let admin_id = env_vars::admin_id().parse::<i32>()?;
	let bearer_token = env_vars::bearer_token();

	let client_id = env_vars::fimfic_client_id();
	let oauth_redirect_url = env_vars::fimfic_oauth_redirect_url();
	let login_url = fimfic_cfg::make_login_url(&client_id, &oauth_redirect_url);
	let fimfic_cfg = FimficCfg::builder()
		.client_id(client_id)
		.client_secret(env_vars::fimfic_client_secret())
		.oauth_redirect_url(oauth_redirect_url)
		.login_url(login_url)
		.bearer_token(bearer_token.clone())
		.build();
	let fimfic = Data(fimfic_cfg);

	let http_client = HttpClient::new().await?;
	let http_client = Data(http_client);

	let admin = match db.get_user_opt(admin_id).await? {
		Some(admin) => admin,
		None => {
			let admin = http_client.get_fimfic_user(admin_id, &bearer_token).await?;
			db.insert_user(admin_id, &admin.data).await?
		}
	};

	let credentials = Credentials::new(
		env_vars::bucket_access_key_id(),
		env_vars::bucket_secret_access_key(),
		None,
		None,
		"r2",
	);

	let s3config = Config::builder()
		.behavior_version_latest()
		.region(Region::new("auto"))
		.credentials_provider(credentials)
		.endpoint_url(format!(
			"https://{}.us.r2.cloudflarestorage.com",
			env_vars::cloudflare_account_id()
		))
		.build();

	let s3client = Client::from_conf(s3config);
	let s3client = Data(s3client);

	let create_dev_session = env_vars::create_dev_session().is_some();
	let token = rand::gen_auth_token();

	let dev_session = create_dev_session.then(|| {
		auth::DevSession::new(
			token.clone(),
			admin_id,
			admin
				.pfp_url
				.unwrap_or_else(|| "https://static.fimfiction.net/images/none_64.png".into()),
		)
	});
	let dev_session = Data(dev_session);

	println!("listening at http://localhost:{PORT}");

	if create_dev_session {
		println!();
		println!("You should unset the `CREATE_DEV_SESSION` environment variable in production.");
		println!(
			"to set a development session, open this link in your browser: http://localhost:{PORT}/dev-session/{token}"
		);
	}

	let server = HttpServer::new(move || {
		ActixApp::new()
			.service(oembed)
			.service(get_css)
			.service(get_js)
			.service(get_home)
			.service(auth::fimfic_auth)
			.service(auth::fimfic_auth_logout)
			.service(set_revoke_sessions)
			.service(set_delete_user)
			.service(set_update_user)
			.service(set_add_user)
			.service(get_user)
			.service(get_comics)
			.service(get_manage_comic)
			.service(set_new_comic)
			.service(set_rename_comic)
			.service(auth::dev_session)
			.service(Files::new("/assets", "./assets"))
			.app_data(db.clone())
			.app_data(fimfic.clone())
			.app_data(http_client.clone())
			.app_data(s3client.clone())
			.app_data(dev_session.clone())
			.wrap(Compress::default())
	});

	server.bind(("0.0.0.0", PORT))?.run().await?;

	Ok(())
}
