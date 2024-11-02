use chrono::{Duration, NaiveTime, Utc};

#[macro_export]
macro_rules! t {
    (now) => {{
        Utc::now()
    }};

    ($hour:literal : $minute:literal) => {{
        NaiveTime::from_hms_opt($hour, $minute, 0).unwrap()
    }};

    ($amount:literal seconds) => {{
        Duration::seconds($amount)
    }};
    ($amount:literal minutes) => {{
        Duration::minutes($amount)
    }};
    ($amount:literal hours) => {{
        Duration::hours($amount)
    }};

    ($amount:literal seconds ago) => {{
        Utc::now() - t!($amount seconds)
    }};
    ($amount:literal minutes ago) => {{
        Utc::now() - t!($amount minutes)
    }};
    ($amount:literal hours ago) => {{
        Utc::now() - t!($amount hours)
    }};
}

#[cfg(test)]
mod tests {
    use chrono::{Timelike, Utc};

    use super::*;

    #[test]
    fn test_time() {
        let dt = Utc::now().with_time(t!(5:34)).earliest().unwrap();

        assert_eq!(dt.hour(), 5);
        assert_eq!(dt.minute(), 34);
    }

    #[test]
    fn test_duration_seconds() {
        let duration = t!(10 seconds);

        assert_eq!(duration.num_seconds(), 10);
    }

    #[test]
    fn test_duration_minutes() {
        let duration = t!(10 minutes);

        assert_eq!(duration.num_minutes(), 10);
    }

    #[test]
    fn test_duration_hours() {
        let duration = t!(10 hours);

        assert_eq!(duration.num_hours(), 10);
    }

    #[test]
    fn test_duration_seconds_ago() {
        let now = Utc::now();
        let dt = t!(10 seconds ago);

        assert!(now >= dt);
        assert!(now - Duration::seconds(10) <= dt);
    }

    #[test]
    fn test_duration_minutes_ago() {
        let now = Utc::now();
        let dt = t!(10 minutes ago);

        assert!(now >= dt);
        assert!(now - Duration::minutes(10) <= dt);
    }

    #[test]
    fn test_duration_hours_ago() {
        let now = Utc::now();
        let dt = t!(10 hours ago);

        assert!(now >= dt);
        assert!(now - Duration::hours(10) <= dt);
    }
}
