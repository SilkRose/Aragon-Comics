use crate::error::Result;
use crate::structs::*;
use pony::fimfiction_api::user::UserData;
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

fn insert_err(err: sqlx::Error) -> String {
	format!("database insertion error:\n{err}")
}

fn select_err(err: sqlx::Error) -> String {
	format!("database selection error:\n{err}")
}

fn update_err(err: sqlx::Error) -> String {
	format!("database updating error:\n{err}")
}

fn delete_err(err: sqlx::Error) -> String {
	format!("database deletion error:\n{err}")
}

fn count_err() -> &'static str {
	"database counting error"
}

fn db_expect() -> &'static str {
	"database constraints means this resource will always be present in the database."
}

#[derive(Clone)]
pub struct Db {
	pool: Pool<Postgres>,
}

impl Db {
	pub async fn new(database_url: &str) -> Result<Self> {
		let pool = PgPoolOptions::new()
			.max_connections(16)
			.connect(database_url)
			.await?;

		sqlx::migrate!().run(&pool).await?;

		Ok(Self { pool })
	}

	pub async fn transaction(&self) -> Result<DbTransaction<'_>> {
		let tx = self.pool.begin().await?;
		Ok(DbTransaction { tx })
	}
}

impl DbExecutor for Db {
	type Executor<'c> = &'c Pool<Postgres>;

	fn executor(&mut self) -> &Pool<Postgres> {
		&self.pool
	}
}

pub struct DbTransaction<'c> {
	tx: sqlx::Transaction<'c, Postgres>,
}

impl<'c> DbTransaction<'c> {
	async fn commit(self) -> Result<()> {
		self.tx.commit().await?;
		Ok(())
	}
}

impl<'c> DbExecutor for DbTransaction<'c> {
	type Executor<'c2>
		= &'c2 mut sqlx::PgConnection
	where
		Self: 'c2;

	fn executor(&mut self) -> &mut sqlx::PgConnection {
		&mut self.tx
	}
}

#[expect(
	async_fn_in_trait,
	reason = "we don't need any implemented auto traits"
)]
pub trait DbExecutor {
	type Executor<'c>: sqlx::Executor<'c, Database = Postgres>
	where
		Self: 'c;

	fn executor(&mut self) -> Self::Executor<'_>;

	async fn insert_user(&mut self, id: i32, data: &UserData<i32>) -> Result<User> {
		Ok(sqlx::query_as!(
			User,
			r#"INSERT INTO Users
				(id, name, pfp_url)
			VALUES
				($1, $2, $3)
			ON CONFLICT(id) DO UPDATE SET
				name = EXCLUDED.name,
				pfp_url = EXCLUDED.pfp_url
			RETURNING
				id, name, pfp_url;"#,
			id,
			data.attributes.name.clone(),
			(!data.attributes.avatar.r64.ends_with("none_64.png")).then_some(
				data.attributes
					.avatar
					.r256
					.trim_end_matches("-256")
					.to_string(),
			),
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	async fn get_user_opt(&mut self, id: i32) -> Result<Option<User>> {
		Ok(sqlx::query_as!(
			User,
			r#"SELECT
				id, name, pfp_url
			FROM Users WHERE id = $1 LIMIT 1;"#,
			id,
		)
		.fetch_optional(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn get_user(&mut self, id: i32) -> Result<User> {
		Ok(self.get_user_opt(id).await?.ok_or_else(db_expect)?)
	}

	async fn delete_user(&mut self, user_id: i32) -> Result<u64> {
		Ok(sqlx::query!("DELETE FROM Users WHERE id = $1;", user_id)
			.execute(self.executor())
			.await
			.map_err(delete_err)?
			.rows_affected())
	}

	async fn get_all_users(&mut self) -> Result<Vec<User>> {
		Ok(sqlx::query_as!(
			User,
			r#"SELECT
				id, name, pfp_url
			FROM Users
			ORDER BY name;"#,
		)
		.fetch_all(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn insert_session(
		&mut self, token: &str, user_id: i32, user_agent: &str,
	) -> Result<Session> {
		Ok(sqlx::query_as!(
			Session,
			"INSERT INTO Tokens
				(token, user_id, user_agent)
			VALUES
				($1, $2, $3)
			RETURNING
				token, user_id, user_agent, last_seen, date_created;",
			token,
			user_id,
			user_agent
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	/// Use when you need to get the session without updating the last seen time.
	async fn get_session_by_token(&mut self, token: &str) -> Result<Option<Session>> {
		Ok(sqlx::query_as!(
			Session,
			"SELECT token, user_id, user_agent, last_seen, date_created
			FROM Tokens WHERE token = $1;",
			token
		)
		.fetch_optional(self.executor())
		.await
		.map_err(select_err)?)
	}

	/// Use when you need to get the session and update the last seen time.
	async fn update_session_last_seen(&mut self, token: &str) -> Result<Option<Session>> {
		Ok(sqlx::query_as!(
			Session,
			"UPDATE Tokens SET last_seen = now() WHERE token = $1
			RETURNING
				token, user_id, user_agent, last_seen, date_created;",
			token
		)
		.fetch_optional(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn get_all_user_sessions(&mut self, user_id: i32) -> Result<Vec<Session>> {
		Ok(sqlx::query_as!(
			Session,
			"SELECT
				token, user_id, user_agent, last_seen, date_created
			FROM Tokens
			WHERE user_id = $1;",
			user_id
		)
		.fetch_all(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn delete_session(&mut self, token: &str) -> Result<u64> {
		Ok(sqlx::query!("DELETE FROM Tokens WHERE token = $1;", token)
			.execute(self.executor())
			.await
			.map_err(delete_err)?
			.rows_affected())
	}

	async fn delete_sessions_by_user_id(&mut self, user_id: i32) -> Result<u64> {
		Ok(
			sqlx::query!("DELETE FROM Tokens WHERE user_id = $1;", user_id)
				.execute(self.executor())
				.await
				.map_err(delete_err)?
				.rows_affected(),
		)
	}

	async fn insert_comic(&mut self, title: &str, url_stub: &str) -> Result<Comic> {
		Ok(sqlx::query_as!(
			Comic,
			"INSERT INTO Comics
				(title, url_stub)
			VALUES
				($1, $2)
			RETURNING
				id, title, url_stub, page_hits, date_modified, date_created;",
			title,
			url_stub
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	async fn get_comic_by_id(&mut self, id: i32) -> Result<Comic> {
		Ok(sqlx::query_as!(
			Comic,
			"SELECT
				id, title, url_stub, page_hits, date_modified, date_created
			FROM Comics
			WHERE id = $1;",
			id
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	async fn get_comic_by_url(&mut self, url_stub: &str) -> Result<Comic> {
		Ok(sqlx::query_as!(
			Comic,
			"SELECT
				id, title, url_stub, page_hits, date_modified, date_created
			FROM Comics
			WHERE url_stub = $1;",
			url_stub
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	async fn get_comic_by_url_count_hit(&mut self, url_stub: &str) -> Result<Comic> {
		Ok(sqlx::query_as!(
			Comic,
			"UPDATE Comics SET page_hits = page_hits + 1
			WHERE url_stub = $1
			RETURNING
				id, title, url_stub, page_hits, date_modified, date_created;",
			url_stub
		)
		.fetch_one(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn get_all_comics(&mut self) -> Result<Vec<Comic>> {
		Ok(sqlx::query_as!(
			Comic,
			"SELECT
				id, title, url_stub, page_hits, date_modified, date_created
			FROM Comics
			ORDER BY date_created DESC;",
		)
		.fetch_all(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn update_comic_title(
		&mut self, old_stub: &str, new_stub: &str, new_title: &str,
	) -> Result<Comic> {
		Ok(sqlx::query_as!(
			Comic,
			"UPDATE Comics
			SET
				url_stub = $2,
				title = $3,
				date_modified = now()
			WHERE url_stub = $1
			RETURNING
				id, title, url_stub, page_hits, date_modified, date_created;",
			old_stub,
			new_stub,
			new_title
		)
		.fetch_one(self.executor())
		.await
		.map_err(update_err)?)
	}

	async fn delete_comic(&mut self, id: i32) -> Result<u64> {
		Ok(sqlx::query!("DELETE FROM Comics WHERE id = $1;", id)
			.execute(self.executor())
			.await
			.map_err(delete_err)?
			.rows_affected())
	}

	async fn insert_panel(
		&mut self, comic_id: i32, panel_number: Decimal, bytes_original: i32, bytes_compressed: i32,
	) -> Result<Panel> {
		Ok(sqlx::query_as!(
			Panel,
			"INSERT INTO Panels
				(comic_id, panel_number, bytes_original, bytes_compressed)
			VALUES
				($1, $2, $3, $4)
			RETURNING
				comic_id, panel_number, panel_revision, bytes_original,
				bytes_compressed, date_modified, date_created;",
			comic_id,
			panel_number,
			bytes_original,
			bytes_compressed
		)
		.fetch_one(self.executor())
		.await
		.map_err(insert_err)?)
	}

	async fn get_panel(&mut self, comic_id: i32, panel_number: Decimal) -> Result<Panel> {
		Ok(sqlx::query_as!(
			Panel,
			"SELECT
				comic_id, panel_number, panel_revision, bytes_original,
				bytes_compressed, date_modified, date_created
			FROM Panels
			WHERE
				comic_id = $1
			AND
				panel_number = $2;",
			comic_id,
			panel_number
		)
		.fetch_one(self.executor())
		.await
		.map_err(delete_err)?)
	}

	async fn get_all_panels_by_comic(&mut self, comic_id: i32) -> Result<Vec<Panel>> {
		Ok(sqlx::query_as!(
			Panel,
			"SELECT
				comic_id, panel_number, panel_revision, bytes_original,
				bytes_compressed, date_modified, date_created
			FROM Panels
			WHERE comic_id = $1
			ORDER BY panel_number ASC;",
			comic_id
		)
		.fetch_all(self.executor())
		.await
		.map_err(select_err)?)
	}

	async fn update_panel_revision(
		&mut self, comic_id: i32, panel_number: Decimal, bytes_original: i32, bytes_compressed: i32,
	) -> Result<Panel> {
		Ok(sqlx::query_as!(
			Panel,
			"UPDATE Panels
			SET
				panel_revision = panel_revision + 1,
				bytes_original = $3,
				bytes_compressed = $4,
				date_modified = now()
			WHERE
				comic_id = $1
			AND
				panel_number = $2
			RETURNING
				comic_id, panel_number, panel_revision, bytes_original,
				bytes_compressed, date_modified, date_created;",
			comic_id,
			panel_number,
			bytes_original,
			bytes_compressed
		)
		.fetch_one(self.executor())
		.await
		.map_err(update_err)?)
	}

	async fn delete_panel(&mut self, comic_id: i32, panel_number: Decimal) -> Result<u64> {
		Ok(sqlx::query!(
			"DELETE FROM Panels
			WHERE
				comic_id = $1
			AND
				panel_number = $2;",
			comic_id,
			panel_number
		)
		.execute(self.executor())
		.await
		.map_err(delete_err)?
		.rows_affected())
	}
}
