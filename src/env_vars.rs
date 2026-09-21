use std::env::{self, VarError};

pub fn load_dotenv() {
	if let Err(err) = dotenvy::dotenv() {
		eprintln!("dotenv failed to load: {err:?}")
	}
}

macro_rules! declare_env_fn {
	{
		$(
			$(#[$meta:meta])*
			$(($optional:ident))? $fn_name:ident() -> $key:literal
		)*
	} => {
		pub fn check() {
			$(
				let _ = $fn_name();
			)*
		}

		$(declare_env_fn! { @helper $(($optional))? $fn_name $key })*
	};

	{
		@helper
		$(#[$meta:meta])*
		$fn_name:ident $key:literal
	} => {
		$(#[$meta])*
		pub fn $fn_name() -> Box<str> {
			required_inner($key)
		}
	};
	{
		@helper
		$(#[$meta:meta])*
		(optional) $fn_name:ident $key:literal
	} => {
		$(#[$meta])*
		pub fn $fn_name() -> Option<Box<str>> {
			optional_inner($key)
		}
	};
}

fn required_inner(key: &str) -> Box<str> {
	optional_inner(key).unwrap_or_else(|| panic!("environment variable `{key}` is not set"))
}

fn optional_inner(key: &str) -> Option<Box<str>> {
	match env::var(key) {
		Ok(var) => Some(var.into_boxed_str()),
		Err(VarError::NotPresent) => None,
		Err(VarError::NotUnicode(_)) => {
			panic!("environment variable `{key}` is set, but not valid UTF-8")
		}
	}
}

declare_env_fn! {
	/// URL to use to connect to postgres
	database_url() -> "DATABASE_URL"

	/// fimfic user id for the site admin
	admin_id() -> "ADMIN_ID"

	/// fimfic API key for updating the story
	bearer_token() -> "BEARER_TOKEN"

	/// fimfic oauth2 client id
	fimfic_client_id() -> "FIMFIC_CLIENT_ID"

	/// fimfic oauth2 client secret
	fimfic_client_secret() -> "FIMFIC_CLIENT_SECRET"

	/// fimfic oauth2 redirect url
	fimfic_oauth_redirect_url() -> "FIMFIC_OAUTH_REDIRECT_URL"

	/// If this is set, the application will create and print a session token
	/// for the fimfic user behind `BEARER_TOKEN`
	///
	/// This should not be set in production environments for security reasons
	(optional) create_dev_session() -> "CREATE_DEV_SESSION"
}
