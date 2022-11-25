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
