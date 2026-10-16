use crate::structs::*;
use crate::{SITE_LINK, SITE_NAME};
use bon::builder;
use maud::{DOCTYPE, PreEscaped, html};
use pony::bytes::FormatType::Abbreviation;
use pony::bytes::format_size_bytes;
use pony::number_format::{format_number_i128, format_number_u128};
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
		p {
			"Authorized users can create, manage, and upload comics, so be \
			careful who you give access to this site. Once someone is added \
			to the list below they can login at "
			a href = "/login/fimfic" { "this" sup { "↗" } } " link."
		}
		h3 { "User List:" }
		p { "Click the buttons below to update or remove a user." }
		@for user in users {
			span class = "row" {
			(user_inline_html(&user))
			@let link = format!("/user/update/{}", user.id);
			(button_link("Update", &link))
			@let link = format!("/user/remove/{}", user.id);
			button
				class = "danger"
				onclick = (format!(r#"openDialog("{}")"#, user.id))
					{ "Remove" }
			dialog id = (user.id) closedby = "any" {
				h2 { "Remove User" }
				p {
					"Are you sure you want to remove "
					(user.name)
					" from the authorized users list?"
				}
				span class = "spaced-row" {
					button onclick = (format!(r#"closeDialog("{}")"#, user.id)) { "Close" }
					button
						type = "button"
						class = "danger"
						onclick = (format!("window.location.href='{link}';"))
							{ "Remove" }
				}
			}
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
			td role = "cell" data-cell = "Created: "
			{
				span class = "date" { (session.date_created.format("%y-%m-%d")) }
				span class = "time" { (session.date_created.format("%H:%M")) }
			}
			td role = "cell" data-cell = "Last Seen: "
			{
				span class = "date" { (session.last_seen.format("%y-%m-%d")) }
				span class = "time" { (session.last_seen.format("%H:%M")) }
			}
		}
	)
}

pub fn home_html(user: Option<User>) -> String {
	let heading = "Home";
	let title: String = format!("{heading} - {SITE_NAME}");
	let description = "Nothing to see here…";
	let mane = html! {
		h1 { "Aragón Comics" }
		p { (description) }
	};
	html_builder()
		.head(head_html(&title, description, SITE_LINK))
		.header(header_html(Pages::Home, user.is_some()))
		.mane(mane)
		.call()
}

pub fn comic_html(comics: Vec<(Comic, ComicPanelData)>, data: ComicData) -> String {
	let heading = "Comics";
	let title: String = format!("{heading} - {SITE_NAME}");
	let description = "Comic list, statistics and a new comic form.";
	let link = format!("{SITE_LINK}/comics");
	let mane = html! {
		h1 { (heading) }
		p { (description) }
		h2 { "New Comic" }
		form class = "row" method = "get" action = "/comics/new" {
			label for = "title" { "Title:" }
			(input_text_required("title", "title", 1, 256))
			button type = "submit" { "Create" }
		}
		h2 { "Comic Statistics" }
		p {
			b { "Comics: " }
			(data.comics)
			b { " Panels: " }
			(format_number_u128(data.panels as u128).unwrap())
			b { " Panel Revisions: " }
			(format_number_u128(data.revisions as u128).unwrap()) br;
			b { "Bytes Original/Compressed: " }
			(&format_size_bytes(data.bytes_original as f64, Abbreviation).unwrap())
			"/"
			(&format_size_bytes(data.bytes_compressed as f64, Abbreviation).unwrap())
			b { " Page Hits: " }
			(format_number_i128(data.page_hits).unwrap())
		}
		h2 { "Comic List" }
		@for (comic, panel_data) in comics {
			span class = "list-item" {
				h3 { a href = (format!("/comics/{}", comic.url_stub)) { (comic.title) sup { "↗" } } }
				p {
					b { "Panels: " }
					(panel_data.panel_count)
					b { " Bytes Original/Compressed: " }
					(&format_size_bytes(panel_data.bytes_original as f64, Abbreviation).unwrap())
					"/"
					(&format_size_bytes(panel_data.bytes_compressed as f64, Abbreviation).unwrap())
					b { " Page Hits: " }
					(format_number_u128(comic.page_hits as u128).unwrap())
					" "
					a href = (format!("/comics/manage/{}", comic.url_stub)) { "Manage" sup { "↗" } }
				}
				p {
					b { "Created: " }
					(comic.date_created.format("%y-%m-%d %H:%M"))
					b { " Last Edit: " }
					(comic.date_modified.format("%y-%m-%d %H:%M"))
					@if let Some(date) = panel_data.latest_date {
						b { " Last Panel Edit: " }
						(date.format("%y-%m-%d %H:%M"))
					}
				}
			}
		}
	};
	html_builder()
		.head(head_html(&title, description, &link))
		.header(header_html(Pages::Comics, true))
		.mane(mane)
		.call()
}

pub fn manage_comic_html(comic: Comic, panels: Vec<Panel>) -> String {
	let heading = format!("{} - Manage", comic.title);
	let title: String = format!("{heading} - {SITE_NAME}");
	let description = "Comic and panel management.";
	let link = format!("{SITE_LINK}/comics/manage/{}", comic.url_stub);
	let mane = html! {
		h1 { a href = (comic.link()) { (comic.title) sup { "↗" } } " - Manage" }
		p { (description) }
		h2 { "Rename Comic" }
		p { "Rename the comic. (Updates the link in the process.)" }
		form class = "row" method = "get" action = (format!("/comics/rename/{}", comic.id)) {
			label for = "title" { "Title:" }
			(input_text_required("title", "title", 1, 256))
			button type = "submit" { "Rename" }
		}
		h2 { "Copy Data" }
		p { "Copy relevant data to the comic." }
		span class = "row" {
			button onclick = (format!(r#"copyText("{}")"#, comic.full_link())) { "Copy Link" }
			button onclick = (format!(r#"copyText("{}")"#, comic.title)) { "Copy Title" }
			button onclick = (format!(r#"copyBbcode("{}")"#, comic.id)) { "Copy BBCode" }
		}
		h2 { "Deletions" }
		p { "Delete all panels and/or the comic as a whole." }
		span class = "row" {
			@if !panels.is_empty() {
				button class = "danger" onclick = r#"openDialog("delete-panels")"# { "Delete Panels" }
				dialog id = "delete-panels" closedby = "any" {
					h2 { "Delete Panels" }
					p {
						"Delete all "
						(panels.len())
						" panels for this comic."
					}
					span class = "row" {
						input type = "checkbox" id = "delete-panel-1" name = "delete-panel-1" autocomplete = "off" {}
						label for = "delete-panel-1" { "Are you sure?" }
					}
					span class = "row" {
						input type = "checkbox" id = "delete-panel-2" name = "delete-panel-2" autocomplete = "off" {}
						label for = "delete-panel-2" { "Are you super sure?" }
					}
					span class = "row" {
						input type = "checkbox" id = "delete-panel-3" name = "delete-panel-3" autocomplete = "off" {}
						label for = "delete-panel-3" { "Are you super-duper sure?" }
					}
					span class = "spaced-row" {
						button onclick = r#"closeDialog("delete-panels")"# { "Close" }
						button
							class = "danger"
							onclick = "deletePanels()"
							{ "Delete" }
					}
				}
			}
			button class = "danger" onclick = r#"openDialog("delete-comic")"# { "Delete Comic" }
			dialog id = "delete-comic" closedby = "any" {
				h2 { "Delete Comic" }
				p {
					"Delete the comic and all "
					(panels.len())
					" panels."
				}
				form
					id = "delete-comic-form" method = "post"
					action = (format!{"/comics/manage/{}/delete", comic.id}) {
					span class = "row" {
						input type = "checkbox" id = "delete-1" name = "delete-1" value = "true" autocomplete = "off" {}
						label for = "delete-1" { "Are you sure?" }
					}
					span class = "row" {
						input type = "checkbox" id = "delete-2" name = "delete-2" value = "true" autocomplete = "off" {}
						label for = "delete-2" { "Are you super sure?" }
					}
					span class = "row" {
						input type = "checkbox" id = "delete-3" name = "delete-3" value = "true" autocomplete = "off" {}
						label for = "delete-3" { "Are you super-duper sure?" }
					}
				}
				span class = "spaced-row" {
					button onclick = r#"closeDialog("delete-comic")"# { "Close" }
					button form = "delete-comic-form" class = "danger" { "Delete" }
				}
			}
		}
		h2 { "Upload Panels" }
		p { label { "Select and upload comic panels." } }
		input
			type = "file"
			id = "pony-up"
			accept = "image/png"
			class = "hidden"
			onchange = "previewFiles()"
			autocomplete = "off"
			multiple {}
		span class = "row" {
			button onclick = "selectFiles()" { "Select Files" }
			button
				class = "danger"
				onclick = "clearSelection()"
					{ "Clear Selection" }
			button onclick = "uploadFiles()" { "Upload Files" }
		}
		dialog id = "clear-selection" closedby = "any" {
			h2 { "Clear Selected Files" }
			p {
				"Are you sure you want to clear all "
				span id = "panel-count" {}
				" selected files for upload?"
			}
			span class = "spaced-row" {
				button onclick = r#"closeDialog("clear-selection")"# { "Close" }
				button
					type = "button"
					class = "danger"
					onclick = "clearFiles()"
						{ "Clear" }
			}
		}
		div id = "preview-panels" class = "manage-panels" {}
		h2 { "Active Panels" }
		div id = "active-panels" class = "manage-panels" {
			@for panel in panels {
				div
					id = (format!("active-{}", panel.number))
					data-panel-number = (panel.number)
					class = "panel-div" {
					img class = "panel" src = (format!("https://r2.pony.sh/{}", panel.key())) {}
					div class = "panel-info" {
						p {
							b { "Number: " }
							(panel.number)
							b { " Revision: " }
							(panel.revision)
							b { " Bytes Original/Compressed: " }
							(format_size_bytes(panel.bytes_original as f64, Abbreviation).unwrap())
							"/"
							(format_size_bytes(panel.bytes_compressed as f64, Abbreviation).unwrap())
							br;
							b { "Created: " }
							(&panel.date_created.format("%y-%m-%d %H:%M"))
							b { " Modified: " }
							(&panel.date_modified.format("%y-%m-%d %H:%M"))
						}
						button
							class = "danger"
							onclick = (format!(r#"openDialog("{}")"#, panel.number))
							{ "Delete" }
					}
					dialog id = (panel.number) closedby = "any" {
						h2 { "Delete Panel" }
						img src = (format!("https://r2.pony.sh/{}", panel.key())) {}
						p { "Are you sure you want to delete delete this panel?" }
						span class = "spaced-row" {
							button
								onclick = (format!(r#"closeDialog("{}")"#, panel.number))
								{ "Close" }
							button
								class = "danger"
								onclick = (format!(r#"deletePanel("{}")"#, panel.number))
								{ "Delete" }
						}
					}
				}
			}
		}
		dialog id = "panel-upload" closedby = "none" data-comic-id = (comic.id) {
				h2 { "Automating Panels" }
				img id = "upload-preview" class = "upload-preview" {}
				p { "The panel above is being:" }
				ol {
					li { "Uploaded to the server." }
					li { "Compressed on the server." }
					li { "Uploaded to online storage." }
				}
				label for = "progress" {
					p {
						"Panel Progress: "
						span id = "panel-index" { "1" }
						"/"
						span id = "panel-total" {}
						", "
						span id = "panel-percent" { "0.00" }
						"%"
					}
				}
				progress id = "progress" {} br;
				p {
					"Runtime: "
					span id = "runtime" { "0 seconds" }
				}
				p {
					"Time remaining: "
					span id = "time-left" { "Calculating…" }
				}
				p {
					"Hit the cancel button to stop uploads "
					"after the current panel has completed."
				}
				button
					type = "button"
					class = "danger"
					onclick = "cancelUpload()"
						{ "Cancel" }
			}
	};
	html_builder()
		.head(head_html(&title, description, &link))
		.header(header_html(Pages::Comics, true))
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
			href = { "/oembed?" (encode) }
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
