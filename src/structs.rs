use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct User {
	pub id: i32,
	pub name: String,
	pub pfp_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
	pub token: String,
	pub user_id: i32,
	pub user_agent: String,
	pub last_seen: DateTime<Utc>,
	pub date_created: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Pages {
	Home,
	User,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OEmbed {
	pub r#type: String,
	pub version: f32,
	pub provider_name: String,
	pub provider_url: String,
	pub title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author_url: Option<String>,
	pub cache_age: u32,
	pub html: String,
}
