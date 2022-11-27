use std::fmt::{self, Display, Formatter};

use chrono::Duration;

#[cfg(test)]
mod tests;

pub struct HumanDuration(pub Duration);

pub fn hours_minutes_seconds(duration: Duration) -> (i64, i64, i64) {
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() - 60 * duration.num_hours();
    let seconds = duration.num_seconds() - 60 * duration.num_minutes();

    (hours, minutes, seconds)
}

impl Display for HumanDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (absolute_duration, sign) = if self.0 > Duration::zero() {
            (self.0, "")
        } else {
            (-self.0, "-")
        };

        let (hours, minutes, seconds) = hours_minutes_seconds(absolute_duration);

        write!(f, "{}{}:{:0>2}:{:0>2}", sign, hours, minutes, seconds)
    }
}
