use crate::errors::InternalResult;
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate};
use futures::future;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{trace, warn};

pub struct Goal {
    pub name: String,
    pub time: Duration,
}

#[derive(Serialize)]
struct TogglQuery<'a> {
    user_agent: &'a str,
    workspace_id: &'a str,
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
    pid: u32,
    start: DateTime<FixedOffset>,
    dur: i64,
}

#[derive(Debug)]
struct TogglEntry {
    start: NaiveDate,
    dur: Duration,
}

pub async fn calculate_goals(
    user_id: i32,
    pool: PgPool,
    client: Client,
) -> InternalResult<Vec<Goal>> {
    let record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    let earliest_start = earliest_start_date(user_id, &pool).await?;

    let r = get_raw_toggl_data(
        &record.workspace_id,
        &record.toggl_api_key,
        &earliest_start,
        &client,
    )
    .await?;

    // todo!()

    Ok(vec![Goal {
        name: String::from("Example"),
        time: Duration::hours(1) + Duration::minutes(2) + Duration::seconds(3),
    }])
}

async fn earliest_start_date(user_id: i32, pool: &PgPool) -> InternalResult<NaiveDate> {
    let records = sqlx::query!(
        "SELECT starting_date FROM projects WHERE user_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut earliest = Local::now().date_naive();
    for record in records {
        if record.starting_date < earliest {
            earliest = record.starting_date;
        }
    }

    Ok(earliest)
}

async fn get_raw_toggl_data(
    workspace_id: &str,
    api_token: &str,
    since: &NaiveDate,
    client: &Client,
) -> InternalResult<Vec<TogglEntry>> {
    // Make one call to the API to determine the total number of pages
    let initial_call = call_api(workspace_id, api_token, since, &client, 1).await?;

    let num_pages = initial_call.total_count / initial_call.per_page + 1;

    // Concurrently call the API for all the remaining pages
    let subsequent_calls = future::join_all(
        (2..=num_pages).map(|page| call_api(workspace_id, api_token, since, client, page)),
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

async fn call_api(
    workspace_id: &str,
    api_token: &str,
    since: &NaiveDate,
    client: &Client,
    page: usize,
) -> InternalResult<TogglResponse> {
    trace!(
        "Calling Toggl API (workspace {}, page {})",
        workspace_id,
        page,
    );

    loop {
        let response = client
            .get("https://api.track.toggl.com/reports/api/v2/details")
            .query(&TogglQuery {
                user_agent: "yottaclock",
                workspace_id,
                since,
                page,
            })
            .basic_auth(api_token, Some("api_token"))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!(
                "Got 429 response from Toggl API (workspace {}, page {})",
                workspace_id, page,
            );

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        } else {
            trace!(
                "Got response from Toggl API (workspace {}, page {})",
                workspace_id,
                page,
            );

            break Ok(response.json().await?);
        }
    }
}

impl From<TogglResponseData> for TogglEntry {
    fn from(other: TogglResponseData) -> Self {
        Self {
            start: other.start.date_naive(),
            dur: Duration::milliseconds(other.dur),
        }
    }
}
