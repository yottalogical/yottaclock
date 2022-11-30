use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
};

use chrono::{DateTime, Datelike, Days, Duration, FixedOffset, NaiveDate, Utc, Weekday};
use chrono_tz::Tz;
use futures::future;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{trace, warn};

use crate::{
    errors::{InternalError, InternalResult},
    session::UserKey,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub i64);

impl Display for ProjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceId(pub i64);

impl Display for WorkspaceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
pub struct WhichWeekdays {
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub starting_date: NaiveDate,
    pub daily_goal: Duration,
    pub days_off: HashSet<NaiveDate>,
    pub weekdays: WhichWeekdays,
}

#[derive(Debug, PartialEq)]
struct ProjectWithDebt {
    project: Project,
    debt: Duration,
}

#[derive(Debug, Deserialize)]
pub struct TogglProject {
    active: bool,
    pub name: String,
    pub id: ProjectId,
}

pub async fn get_workspaces(
    toggl_api_token: &str,
    client: Client,
) -> InternalResult<Option<Vec<Workspace>>> {
    loop {
        let response = client
            .get("https://api.track.toggl.com/api/v9/workspaces")
            .basic_auth(toggl_api_token, Some("api_token"))
            .send()
            .await?;

        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            StatusCode::OK => break Ok(response.json().await?),
            _ => break Ok(None),
        }
    }
}

pub async fn get_workspace_details(
    toggl_api_token: &str,
    workspace_id: WorkspaceId,
    client: Client,
) -> InternalResult<Option<Workspace>> {
    let url = format!(
        "https://api.track.toggl.com/api/v9/workspaces/{}",
        workspace_id,
    );

    loop {
        let response = client
            .get(&url)
            .basic_auth(toggl_api_token, Some("api_token"))
            .send()
            .await?;

        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            StatusCode::OK => break Ok(response.json().await?),
            _ => break Ok(None),
        }
    }
}

pub async fn get_toggl_projects(
    toggl_api_token: &str,
    workspace_id: WorkspaceId,
    client: &Client,
) -> InternalResult<Vec<TogglProject>> {
    let url = format!(
        "https://api.track.toggl.com/api/v9/workspaces/{}/projects",
        workspace_id,
    );

    loop {
        let response = client
            .get(&url)
            .basic_auth(toggl_api_token, Some("api_token"))
            .send()
            .await?;

        if let StatusCode::TOO_MANY_REQUESTS = response.status() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        } else {
            let all_projects: Vec<TogglProject> = response.json().await?;

            break Ok(all_projects
                .into_iter()
                .filter(|project| project.active)
                .collect());
        }
    }
}

pub async fn calculate_goals(
    user_key: UserKey,
    pool: PgPool,
    client: Client,
) -> InternalResult<Option<(Vec<Goal>, Duration)>> {
    let record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id, daily_max, timezone
        FROM users
        WHERE user_key = $1",
        user_key.0,
    )
    .fetch_one(&pool)
    .await?;

    let projects = get_user_projects(
        &record.toggl_api_key,
        WorkspaceId(record.workspace_id),
        &client,
        user_key,
        &pool,
    )
    .await?;

    Ok(if !projects.is_empty() {
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

        Some((goals, total_debt))
    } else {
        None
    })
}

pub async fn get_user_projects(
    toggl_api_token: &str,
    workspace_id: WorkspaceId,
    client: &Client,
    user_key: UserKey,
    pool: &PgPool,
) -> InternalResult<HashMap<ProjectId, Project>> {
    let records_future = sqlx::query!(
        "SELECT project_key, project_id, starting_date, daily_goal,
            monday, tuesday, wednesday, thursday, friday, saturday, sunday
        FROM projects
        WHERE user_key = $1",
        user_key.0,
    )
    .fetch_all(pool);
    let toggl_projects_future = get_toggl_projects(toggl_api_token, workspace_id, client);
    let (records, toggl_projects) = future::join(records_future, toggl_projects_future).await;

    let project_id_to_name: HashMap<ProjectId, String> = toggl_projects?
        .into_iter()
        .map(|project| (project.id, project.name))
        .collect();

    let mut futures = Vec::new();
    for record in records? {
        if let Some(project_name) = project_id_to_name.get(&ProjectId(record.project_id)) {
            futures.push(async move {
                let days_off = sqlx::query!(
                    "SELECT days_off.day_off
                    FROM days_off
                    INNER JOIN days_off_to_projects
                    ON days_off.day_off_key = days_off_to_projects.day_off_key
                    WHERE days_off_to_projects.project_key = $1",
                    user_key.0,
                )
                .fetch_all(pool)
                .await?;

                let weekdays = WhichWeekdays {
                    monday: record.monday,
                    tuesday: record.tuesday,
                    wednesday: record.wednesday,
                    thursday: record.thursday,
                    friday: record.friday,
                    saturday: record.saturday,
                    sunday: record.sunday,
                };

                Ok((
                    ProjectId(record.project_id),
                    Project {
                        name: project_name.clone(),
                        starting_date: record.starting_date,
                        daily_goal: Duration::seconds(record.daily_goal),
                        days_off: days_off.into_iter().map(|record| record.day_off).collect(),
                        weekdays,
                    },
                ))
            });
        }
    }

    let future_results: Vec<InternalResult<(ProjectId, Project)>> = future::join_all(futures).await;

    future_results
        .into_iter()
        .try_fold(HashMap::new(), |mut acc, futures_result| {
            let (project_id, project) = futures_result?;
            acc.insert(project_id, project);
            Ok(acc)
        })
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
    let toggl_entires_from_initial_call: Vec<TogglEntry> = initial_call
        .data
        .into_iter()
        .map(TogglResponseData::into)
        .collect();

    // Fold the results from the subsequent calls into the Vec
    subsequent_calls.into_iter().try_fold(
        toggl_entires_from_initial_call,
        |mut acc, subsequent_call| {
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
                user_agent: "yottaclock.com",
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
                    debt: Duration::zero(),
                },
            )
        })
        .collect();

    let mut total_debt = Duration::zero();

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
                total_debt = total_debt - max(min(*debt, entry.duration), Duration::zero());

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

    let mut total_debt = Duration::zero();

    // Increase the debts
    for ProjectWithDebt { project, debt } in projects_with_debts.values_mut() {
        if project.advance_debt_on(*current_date) {
            *debt = *debt + project.daily_goal;
        }

        total_debt = total_debt + *debt;
    }

    // Ensure the total debt doesn't exceed the daily max
    if total_debt > daily_max {
        total_debt = daily_max;
    }

    // If they exceeded their goal yesterday, carry over the extra to today
    if previous_total_debt < Duration::zero() {
        total_debt = total_debt + previous_total_debt;
    }

    total_debt
}

impl Project {
    fn advance_debt_on(&self, date: NaiveDate) -> bool {
        if date < self.starting_date {
            return false;
        }

        if self.days_off.contains(&date) {
            return false;
        }

        match date.weekday() {
            Weekday::Mon if !self.weekdays.monday => return false,
            Weekday::Tue if !self.weekdays.tuesday => return false,
            Weekday::Wed if !self.weekdays.wednesday => return false,
            Weekday::Thu if !self.weekdays.thursday => return false,
            Weekday::Fri if !self.weekdays.friday => return false,
            Weekday::Sat if !self.weekdays.saturday => return false,
            Weekday::Sun if !self.weekdays.sunday => return false,
            _ => (),
        }

        true
    }
}
