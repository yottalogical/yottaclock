-- Add up migration script here
CREATE TABLE users (
    user_key BIGSERIAL PRIMARY KEY,
    toggl_api_key TEXT UNIQUE NOT NULL,
    workspace_id BIGINT NOT NULL,
    daily_max BIGINT NOT NULL,
    timezone TEXT NOT NULL
);

CREATE TABLE projects (
    project_key BIGSERIAL PRIMARY KEY,
    user_key BIGINT NOT NULL REFERENCES users(user_key) ON DELETE CASCADE,
    project_id BIGINT NOT NULL,
    starting_date DATE NOT NULL,
    daily_goal BIGINT NOT NULL,

    monday BOOLEAN NOT NULL,
    tuesday BOOLEAN NOT NULL,
    wednesday BOOLEAN NOT NULL,
    thursday BOOLEAN NOT NULL,
    friday BOOLEAN NOT NULL,
    saturday BOOLEAN NOT NULL,
    sunday BOOLEAN NOT NULL,

    UNIQUE (user_key, project_id)
);

CREATE TABLE days_off (
    day_off_key BIGSERIAL PRIMARY KEY,
    day_off DATE NOT NULL
);

CREATE TABLE days_off_to_projects (
    project_key BIGINT NOT NULL REFERENCES projects(project_key) ON DELETE CASCADE,
    day_off_key BIGINT NOT NULL REFERENCES days_off(day_off_key) ON DELETE CASCADE,
    PRIMARY KEY (project_key, day_off_key)
);

CREATE TABLE session_tokens (
    token TEXT PRIMARY KEY,
    user_key BIGINT NOT NULL REFERENCES users(user_key) ON DELETE CASCADE
);
