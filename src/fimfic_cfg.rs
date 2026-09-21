use bon::bon;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct FimficCfg {
	inner: Arc<FimficCfgInner>,
}

pub struct FimficCfgInner {
	pub client_id: Box<str>,
	pub client_secret: Box<str>,
	pub oauth_redirect_url: Box<str>,
	/// Login URL except missing state (ie. `format!("{url}&state={state}")` to
	/// get a complete URL)
	pub login_url: Box<str>,
	pub bearer_token: Box<str>,
}

#[bon]
impl FimficCfg {
	#[builder]
	pub fn new(
		client_id: Box<str>, client_secret: Box<str>, oauth_redirect_url: Box<str>,
		login_url: Box<str>, bearer_token: Box<str>,
	) -> Self {
		Self {
			inner: Arc::new(FimficCfgInner {
				client_id,
				client_secret,
				oauth_redirect_url,
				login_url,
				bearer_token,
			}),
		}
	}
}

impl Deref for FimficCfg {
	type Target = FimficCfgInner;

	fn deref(&self) -> &FimficCfgInner {
		&self.inner
	}
}

/// Makes a login url, purposefully without scope so we can reuse this and
/// clients can generate their own scope to put on it
pub fn make_login_url(client_id: &str, oauth_redirect_url: &str) -> Box<str> {
	format!("https://www.fimfiction.net/authorize-app?client_id={client_id}&response_type=code&scope=read_user&redirect_uri={oauth_redirect_url}")
		.into_boxed_str()
}
