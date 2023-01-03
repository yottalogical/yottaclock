#![cfg(test)]

use super::*;

#[test]
fn test_positive_human_duration() {
    let duration = Duration::hours(1) + Duration::minutes(2) + Duration::seconds(3);
    assert_eq!(HumanDuration(duration).to_string(), "1:02:03");
}

#[test]
fn test_negative_human_duration() {
    let duration = -Duration::hours(1) - Duration::minutes(2) - Duration::seconds(3);
    assert_eq!(HumanDuration(duration).to_string(), "-1:02:03");
}
