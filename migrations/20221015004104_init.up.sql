-- Add up migration script here
CREATE TABLE users (
    user_id SERIAL PRIMARY KEY
);

CREATE TABLE session_tokens (
    token TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(user_id) ON DELETE CASCADE
);
