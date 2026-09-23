use crate::error::Result;
use crate::structs::*;
use pony::fimfiction_api::user::UserData;
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
}
