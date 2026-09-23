use crate::structs::*;
use crate::{SITE_LINK, SITE_NAME};
use bon::builder;
use maud::{DOCTYPE, PreEscaped, html};
use url::form_urlencoded;

#[builder]
fn html_builder(
	head: PreEscaped<String>, header: PreEscaped<String>, mane: PreEscaped<String>,
) -> String {
	let body_content = html! {
		header { (header) }
		main { (mane) }
	};
	let body = html! {body { (body_content) }};
	html!(
		(DOCTYPE) html lang = "en" {
			head { (head) }
			(body)
		}
	)
	.into()
}

pub fn user_settings_html(users: Vec<User>, sessions: Vec<Session>) -> String {
	let heading = "User Settings";
	let title: String = format!("{heading} - {SITE_NAME}");
	let description = "Authorized users, user updates, & user sessions.";
	let link = format!("{SITE_LINK}/user-settings");
	let mane = html! {
		h1 { (heading) }
		p { (description) }
		(authorized_user_html(users)) hr;
		(user_sessions_html(sessions))
	};
	html_builder()
		.head(head_html(&title, description, &link))
		.header(header_html(Pages::User, true))
		.mane(mane)
		.call()
}

fn authorized_user_html(users: Vec<User>) -> PreEscaped<String> {
	html!(
		h2 { "Authorized Users" }
		p { "Update, remove, & add users." }
		h3 { "User List:" }
		p { "Click the buttons below to update or remove a user." }
		@for user in users {
			span class = "row" {
			(user_inline_html(&user))
			@let link = format!("/user/update/{}", user.id);
			(button_link("Update", &link))
			@let link = format!("/user/remove/{}", user.id);
			button
				type = "button"
				class = "danger"
				onclick = (format!("window.location.href='{link}';"))
					{ "Remove" }
		}
		}
		p { "This site pulls user information from Fimfiction. \
			If anyone updates their name or profile picture on Fimfiction, \
			we have no idea unless you click the button to re-fetch the data." }
		h3 { "Add Authorized User" }
		p { "Add a new authorized user with their Fimfiction ID below:" }
		form class = "row" method = "get" action = "/user/add" {
			label for = "id" { "User ID:" }
			(input_text_numeric_required("id", "id", 1, 6))
			button type = "submit" { "Add" }
		}
	)
}

pub fn user_sessions_html(sessions: Vec<Session>) -> PreEscaped<String> {
	html! {
		form method = "post" action = "/user/revoke-sessions" {
			h1 { "Sessions" }
			table role = "table" {
				caption role = "caption" { "Active User Sessions" }
				thead role = "rowgroup" {
					tr role = "row" {
						th role = "columnheader" { "Revoke?" }
						th role = "columnheader" { "User Agent" }
						th role = "columnheader" { "Created" }
						th role = "columnheader" { "Last Seen" }
					}
				}
				tbody role = "rowgroup" {
					(session_table_row(&sessions[0], 0))
					@for (num, session) in sessions.iter().enumerate().skip(1) {
						(session_table_row(session, num))
					}
				}
			}
			button type = "submit" { "Revoke Sessions" }
		}
	}
}

fn session_table_row(session: &Session, num: usize) -> PreEscaped<String> {
	html! (
		tr role = "row" {
			td role = "cell" data-cell = "Revoke: "
				{ input type = "checkbox" id = (num) name = (num) value = (session.token) {} }
			@if num == 0 {
				td role = "cell" data-cell = "User Agent: " { b { "(Active) " } (session.user_agent) }
			} @else {
				td role = "cell" data-cell = "User Agent: " { (session.user_agent) }
			}
			td role = "cell" data-cell = "Created: " { (session.date_created.format("%y-%m-%d %H:%M")) }
			td role = "cell" data-cell = "Last Seen: " { (session.last_seen.format("%y-%m-%d %H:%M")) }
		}
	)
}

pub fn home_html(user: Option<User>) -> String {
	let heading = "Home";
	let title: String = format!("{heading} - {SITE_NAME}");
	let description = "The Equestrian Census, reimagined.";
	let mane = html! {
		h1 { "Census Consensus" }
		p { (description) }
		p {
			""
		}
	};
	html_builder()
		.head(head_html(&title, description, SITE_LINK))
		.header(header_html(Pages::Home, user.is_some()))
		.mane(mane)
		.call()
}

// HTML components go below this comment:

pub fn head_html(title: &str, description: &str, link: &str) -> PreEscaped<String> {
	html! {
		title { (title) };
		meta charset = "UTF-8";
		meta http-equiv = "X-UA-Compatible" content = "IE=edge";
		meta name = "viewport" content = "width=device-width,initial-scale=1";
		link rel = "stylesheet" crossorigin href = "/style.css";
		link rel = "canonical" href = (link);
		meta property = "og:title" content = (title);
		meta property = "og:description" content = (description);
		// meta property = "og:image" content = { (format!("{SITE_LINK}/assets/cc-light-512.png")) };
		meta property = "og:url" content = (link);
		meta property = "og:type" content = "website";
		meta property = "og:site_name" content = (SITE_NAME);
		@let encode = encode_url(title);
		link
			crossorigin
			rel = "alternate"
			type = "application/json+oembed"
			href = { "https://census.silkrose.dev/oembed?" (encode) }
			title = (title);
		script crossorigin src = "/mane.js" {}

	}
}

fn encode_url(title: &str) -> String {
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1.0");
	encode.append_pair("provider_name", SITE_NAME);
	encode.append_pair("provider_url", SITE_LINK);
	encode.append_pair("title", title);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	encode.finish()
}

fn header_html(page: Pages, admin: bool) -> PreEscaped<String> {
	html!(
		@if admin {
			nav {
				span class = "nav" {
					(header_link_html("/", "Home", page == Pages::Home))
					"•"
					(header_link_html("/user", "User", page == Pages::User))
					"•"
					(header_link_html("/comics", "Comics", page == Pages::Comics))
				}
			}
		}
	)
}

fn header_link_html(link: &str, text: &str, checked: bool) -> PreEscaped<String> {
	html!(
		@if checked {
			a href = (link) class = "underlined" { (text) }
		} @ else {
			a href = (link) { (text) }
		}
	)
}

fn input_text_required(id: &str, name: &str, min: u32, max: u32) -> PreEscaped<String> {
	html!	(
		input
			id = (id)
			type = "text"
			name = (name)
			minlength = (min)
			maxlength = (max)
			required {}
	)
}

fn input_text_value_required(
	id: &str, name: &str, min: u32, max: u32, value: &str,
) -> PreEscaped<String> {
	html!	(
		input
			id = (id)
			type = "text"
			name = (name)
			minlength = (min)
			maxlength = (max)
			value = (value)
			required {}
	)
}

fn input_text_numeric_required(id: &str, name: &str, min: u32, max: u32) -> PreEscaped<String> {
	html!	(
		input
			id = (id)
			type = "text"
			name = (name)
			inputmode = "numeric"
			pattern = r"\d*"
			minlength = (min)
			maxlength = (max)
			required {}
	)
}

fn button_link(text: &str, endpoint: &str) -> PreEscaped<String> {
	html! (
		button type = "button" onclick = (format!("window.location.href='{endpoint}';")) { (text) }
	)
}

fn user_inline_html(user: &User) -> PreEscaped<String> {
	html! {
		@if let Some(pfp_url) = &user.pfp_url {
			img src = (format!("{pfp_url}-32")) alt = (user.name) {}
			" - "
		}
		a href = (format!("https://www.fimfiction.net/user/{}/", user.id)) { (user.name) sup { "↗" } }
	}
}
