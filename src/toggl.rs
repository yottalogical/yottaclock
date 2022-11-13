use crate::templates::HumanDuration;
use chrono::Duration;
use sqlx::PgPool;

pub struct Goal {
    pub name: String,
    pub time: HumanDuration,
}

pub fn calculate_goals(user_id: i32, pool: PgPool) -> Vec<Goal> {
    vec![Goal {
        name: String::from("Example"),
        time: HumanDuration(Duration::hours(1) + Duration::minutes(2) + Duration::seconds(3)),
    }]
}
