use crate::toggl::Goal;
use askama::Template;
use chrono::Duration;
use std::fmt::{self, Display, Formatter};

pub struct HumanDuration(pub Duration);

impl Display for HumanDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let hours = self.0.num_hours();
        let minutes = self.0.num_minutes() - 60 * self.0.num_hours();
        let seconds = self.0.num_seconds() - 60 * self.0.num_minutes();

        write!(f, "{}:{:0>2}:{:0>2}", hours, minutes, seconds)
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub user_id: i32,
    pub goals: Vec<Goal>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {}
