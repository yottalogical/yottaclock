-- Add up migration script here
CREATE TABLE users (
    user_id BIGSERIAL PRIMARY KEY,
    toggl_api_key TEXT UNIQUE NOT NULL,
    workspace_id BIGINT NOT NULL,
    daily_max BIGINT NOT NULL,
    timezone TEXT NOT NULL
);

CREATE TABLE projects (
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    project_name TEXT NOT NULL,
    project_id BIGINT NOT NULL,
    starting_date DATE NOT NULL,
    daily_goal BIGINT NOT NULL
);

CREATE TABLE session_tokens (
    token TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE
);
