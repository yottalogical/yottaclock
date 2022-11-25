-- Add up migration script here
CREATE TABLE users (
    user_id SERIAL PRIMARY KEY,
    toggl_api_key TEXT UNIQUE NOT NULL,
    workspace_id TEXT NOT NULL,
    daily_max BIGINT NOT NULL,
    timezone TEXT NOT NULL
);

CREATE TABLE projects (
    user_id INTEGER NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    project_name TEXT NOT NULL,
    project_id INTEGER NOT NULL,
    starting_date DATE NOT NULL,
    daily_goal BIGINT NOT NULL
);

CREATE TABLE session_tokens (
    token TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(user_id) ON DELETE CASCADE
);
