CREATE TABLE IF NOT EXISTS Users (
	id      integer NOT NULL PRIMARY KEY,
	name    text    NOT NULL,
	pfp_url text    NULL
);

CREATE TABLE IF NOT EXISTS Tokens (
	token        text        NOT NULL PRIMARY KEY,
	user_id      integer     NOT NULL,
	user_agent   text        NOT NULL,
	last_seen    timestamptz NOT NULL DEFAULT now(),
	date_created timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT Tokens_Users_fk FOREIGN KEY (user_id)
		REFERENCES Users (id) ON DELETE CASCADE
);
