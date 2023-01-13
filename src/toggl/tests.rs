#![cfg(test)]

use super::*;

#[test]
fn test_total_debt_overflow() {
    let project_id = ProjectId(1234);

    let toggl_entries = vec![TogglEntry {
        project_id,
        date: NaiveDate::from_ymd_opt(2000, 1, 9).unwrap(),
        duration: Duration::hours(4),
    }];

    let example_project = Project {
        name: String::from("Example Project"),
        starting_date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        daily_goal: Duration::hours(1),
        days_off: HashSet::new(),
        weekdays: WhichWeekdays {
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true,
        },
    };

    let user_projects = HashMap::from([(project_id, example_project.clone())]);

    let daily_max = Duration::hours(3);

    let today = NaiveDate::from_ymd_opt(2000, 1, 10).unwrap();

    let (r1, r2) = process_toggl_data(toggl_entries, user_projects, daily_max, today);

    assert_eq!(
        r1,
        HashMap::from([(
            project_id,
            ProjectWithDebt {
                project: example_project,
                debt: Duration::hours(6),
            }
        )])
    );

    assert_eq!(r2, Duration::hours(2));
}

#[test]
fn test_days_off() {
    let project_id = ProjectId(185068848);

    let toggl_entries = Vec::new();

    let example_project = Project {
        name: String::from("EECS 575"),
        starting_date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        daily_goal: Duration::hours(1),
        days_off: HashSet::from([NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()]),
        weekdays: WhichWeekdays {
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true,
        },
    };

    let user_projects = HashMap::from([(project_id, example_project.clone())]);

    let daily_max = Duration::hours(5);

    let today = NaiveDate::from_ymd_opt(2000, 1, 2).unwrap();

    let (r1, r2) = process_toggl_data(toggl_entries, user_projects, daily_max, today);

    assert_eq!(
        r1,
        HashMap::from([(
            project_id,
            ProjectWithDebt {
                project: example_project,
                debt: Duration::hours(1),
            }
        )])
    );

    assert_eq!(r2, Duration::hours(1));
}

#[sqlx::test]
async fn test_get_user_projects(pool: PgPool) {
    let user_key = UserKey(1);

    let project_key = 2;
    let project_id = ProjectId(12345678);
    let project = Project {
        name: String::from("Example Project"),
        starting_date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        daily_goal: Duration::hours(1),
        days_off: HashSet::from([NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()]),
        weekdays: WhichWeekdays {
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true,
        },
    };

    let day_off_key = 3;

    sqlx::query!(
        "INSERT INTO users(user_key, toggl_api_key, workspace_id, daily_max, timezone)
        VALUES ($1, '1971800d4d82861d8f2c1651fea4d212', 1234567, 3600, 'UTC')",
        user_key.0,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO projects(project_key, user_key, project_id, starting_date, daily_goal,
            monday, tuesday, wednesday, thursday, friday, saturday, sunday)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        project_key,
        user_key.0,
        project_id.0,
        project.starting_date,
        project.daily_goal.num_seconds(),
        project.weekdays.monday,
        project.weekdays.tuesday,
        project.weekdays.wednesday,
        project.weekdays.thursday,
        project.weekdays.friday,
        project.weekdays.saturday,
        project.weekdays.sunday,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO days_off(day_off_key, user_key, day_off)
        VALUES ($1, $2, '2000-01-01')",
        day_off_key,
        user_key.0,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO days_off_to_projects(project_key, day_off_key)
        VALUES ($1, $2)",
        project_key,
        day_off_key,
    )
    .execute(&pool)
    .await
    .unwrap();

    let toggl_projects = vec![TogglProject {
        active: true,
        name: String::from("Example Project"),
        id: project_id,
    }];

    let projects = get_user_projects_from_toggl_projects(toggl_projects, user_key, &pool)
        .await
        .unwrap();

    assert_eq!(projects, HashMap::from([(project_id, project)]));
}
