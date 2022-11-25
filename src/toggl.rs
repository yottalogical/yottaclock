use std::{
    cmp::{max, min},
    collections::HashMap,
};

use chrono::{DateTime, Days, Duration, FixedOffset, NaiveDate, Utc};
use chrono_tz::Tz;
use futures::future;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{trace, warn};

use crate::{
    errors::{InternalError, InternalResult},
    session::UserId,
};

#[cfg(test)]
mod tests;

pub struct Goal {
    pub name: String,
    pub time: Duration,
}

#[derive(Serialize)]
struct TogglQuery<'a> {
    user_agent: &'a str,
    workspace_id: i64,
    since: &'a NaiveDate,
    page: usize,
}

#[derive(Debug, Deserialize)]
struct TogglResponse {
    total_count: usize,
    per_page: usize,
    data: Vec<TogglResponseData>,
}

#[derive(Debug, Deserialize)]
struct TogglResponseData {
    pid: i64,
    start: DateTime<FixedOffset>,
    dur: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ProjectId(i64);

#[derive(Clone, Copy)]
struct WorkspaceId(i64);

#[derive(Debug)]
struct TogglEntry {
    project_id: ProjectId,
    date: NaiveDate,
    duration: Duration,
}

impl From<TogglResponseData> for TogglEntry {
    fn from(response_data: TogglResponseData) -> Self {
        Self {
            project_id: ProjectId(response_data.pid),
            date: response_data.start.date_naive(),
            duration: Duration::milliseconds(response_data.dur),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Project {
    name: String,
    starting_date: NaiveDate,
    daily_goal: Duration,
}

#[derive(Debug, PartialEq)]
struct ProjectWithDebt {
    project: Project,
    debt: Duration,
}

pub async fn calculate_goals(
    user_id: UserId,
    pool: PgPool,
    client: Client,
) -> InternalResult<(Vec<Goal>, Duration)> {
    let record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id, daily_max, timezone FROM users WHERE user_id = $1",
        user_id.0,
    )
    .fetch_one(&pool)
    .await?;

    let projects = get_user_projects(user_id, &pool).await?;

    let earliest_start = earliest_start_date(&projects);

    let toggl_entries = get_raw_toggl_data(
        WorkspaceId(record.workspace_id),
        &record.toggl_api_key,
        &earliest_start,
        &client,
    )
    .await?;

    let (project_debts, total_debt) = process_toggl_data(
        toggl_entries,
        projects,
        Duration::seconds(record.daily_max),
        today_in_timezone(&record.timezone)?,
    );

    let mut goals: Vec<Goal> = project_debts
        .into_iter()
        .map(|(_, ProjectWithDebt { project, debt })| Goal {
            name: project.name,
            time: debt,
        })
        .collect();
    goals.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    Ok((goals, total_debt))
}

async fn get_user_projects(
    user_id: UserId,
    pool: &PgPool,
) -> InternalResult<HashMap<ProjectId, Project>> {
    let query = sqlx::query!(
        "SELECT project_id, project_name, starting_date, daily_goal FROM projects WHERE user_id = $1",
        user_id.0,
    );

    Ok(query
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|record| {
            (
                ProjectId(record.project_id),
                Project {
                    name: record.project_name,
                    starting_date: record.starting_date,
                    daily_goal: Duration::seconds(record.daily_goal),
                },
            )
        })
        .collect())
}

fn earliest_start_date(projects: &HashMap<ProjectId, Project>) -> NaiveDate {
    let mut earliest = NaiveDate::MAX;

    for project in projects.values() {
        if project.starting_date < earliest {
            earliest = project.starting_date;
        }
    }

    earliest
}

async fn get_raw_toggl_data(
    workspace_id: WorkspaceId,
    api_token: &str,
    since: &NaiveDate,
    client: &Client,
) -> InternalResult<Vec<TogglEntry>> {
    // Make one call to the API to determine the total number of pages
    let initial_call = call_toggl_api(workspace_id, api_token, since, client, 1).await?;

    let num_pages = initial_call.total_count / initial_call.per_page + 1;

    // Concurrently call the API for all the remaining pages
    let subsequent_calls = future::join_all(
        (2..=num_pages).map(|page| call_toggl_api(workspace_id, api_token, since, client, page)),
    )
    .await;

    // Collect the results from the initial call into a Vec
    let toggl_entires_from_initial_call = initial_call
        .data
        .into_iter()
        .map(TogglResponseData::into)
        .collect();

    // Fold the results from the subsequent calls into the Vec
    subsequent_calls.into_iter().fold(
        Ok(toggl_entires_from_initial_call),
        |acc, subsequent_call| {
            let mut acc = acc?;
            acc.extend(
                subsequent_call?
                    .data
                    .into_iter()
                    .map(TogglResponseData::into),
            );
            Ok(acc)
        },
    )
}

async fn call_toggl_api(
    workspace_id: WorkspaceId,
    api_token: &str,
    since: &NaiveDate,
    client: &Client,
    page: usize,
) -> InternalResult<TogglResponse> {
    trace!(
        "Calling Toggl API (workspace {}, page {})",
        workspace_id.0,
        page,
    );

    loop {
        let response = client
            .get("https://api.track.toggl.com/reports/api/v2/details")
            .query(&TogglQuery {
                user_agent: "yottaclock",
                workspace_id: workspace_id.0,
                since,
                page,
            })
            .basic_auth(api_token, Some("api_token"))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!(
                "Got 429 response from Toggl API (workspace {}, page {})",
                workspace_id.0, page,
            );

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        } else {
            trace!(
                "Got response from Toggl API (workspace {}, page {})",
                workspace_id.0,
                page,
            );

            break Ok(response.json().await?);
        }
    }
}

fn today_in_timezone(timezone: &str) -> InternalResult<NaiveDate> {
    let tz: Tz = timezone
        .parse()
        .map_err(InternalError::UnrecognizedTimezone)?;

    Ok(Utc::now().with_timezone(&tz).date_naive())
}

fn process_toggl_data(
    toggl_entries: Vec<TogglEntry>,
    projects: HashMap<ProjectId, Project>,
    daily_max: Duration,
    today: NaiveDate,
) -> (HashMap<ProjectId, ProjectWithDebt>, Duration) {
    // Sort the entries by their date
    let sorted_toggl_entries = {
        let mut v = toggl_entries;
        v.sort_by_key(|entry| entry.date);
        v
    };

    let mut current_date = earliest_start_date(&projects) - Days::new(1);

    let mut projects_with_debts: HashMap<ProjectId, ProjectWithDebt> = projects
        .into_iter()
        .map(|(project_id, project)| {
            (
                project_id,
                ProjectWithDebt {
                    project,
                    debt: Duration::seconds(0),
                },
            )
        })
        .collect();

    let mut total_debt = Duration::seconds(0);

    for entry in sorted_toggl_entries {
        // Increment the current date until it's caught up to this entry
        while current_date < entry.date {
            total_debt = advance_debt(
                &mut projects_with_debts,
                &mut current_date,
                total_debt,
                daily_max,
            );
        }

        if current_date != entry.date {
            warn!(
                "current_date is {}, but entry.start is {}",
                current_date, entry.date,
            );
        }

        // Subtract this entry from the project debt and the total debt
        if let Some(ProjectWithDebt { project, debt }) =
            projects_with_debts.get_mut(&entry.project_id)
        {
            if project.starting_date <= entry.date {
                // Only subtract from the total debt while the project debt is positive
                total_debt = total_debt - max(min(*debt, entry.duration), Duration::seconds(0));

                *debt = *debt - entry.duration;
            }
        }
    }

    // Increment the current date the rest of the way
    while current_date < today {
        total_debt = advance_debt(
            &mut projects_with_debts,
            &mut current_date,
            total_debt,
            daily_max,
        );
    }

    (projects_with_debts, total_debt)
}

fn advance_debt(
    projects_with_debts: &mut HashMap<ProjectId, ProjectWithDebt>,
    current_date: &mut NaiveDate,
    previous_total_debt: Duration,
    daily_max: Duration,
) -> Duration {
    // Increment the current date
    *current_date = *current_date + Days::new(1);

    let mut total_debt = Duration::seconds(0);

    // Increase the debts
    for ProjectWithDebt { project, debt } in projects_with_debts.values_mut() {
        if *current_date >= project.starting_date {
            *debt = *debt + project.daily_goal;
        }

        total_debt = total_debt + *debt;
    }

    // Ensure the total debt doesn't exceed the daily max
    if total_debt > daily_max {
        total_debt = daily_max;
    }

    // If they exceeded their goal yesterday, carry over the extra to today
    if previous_total_debt < Duration::seconds(0) {
        total_debt = total_debt + previous_total_debt;
    }

    total_debt
}
