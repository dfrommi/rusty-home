#[macro_export]
macro_rules! t {
    (now) => {{
        chrono::Utc::now()
    }};

    ($hour:literal : $minute:literal) => {{
        chrono::NaiveTime::from_hms_opt($hour, $minute, 0).unwrap()
    }};

    ($from_hour:literal : $from_minute:literal - $to_hour:literal : $to_minute:literal) => {{
        support::time::DailyTimeRange::new(
            ($from_hour, $from_minute),
            ($to_hour, $to_minute)
        )
    }};

    ($amount:literal seconds) => {{
        chrono::Duration::seconds($amount)
    }};
    ($amount:literal minutes) => {{
        chrono::Duration::minutes($amount)
    }};
    ($amount:literal hours) => {{
        chrono::Duration::hours($amount)
    }};

    ($amount:literal seconds ago) => {{
        chrono::Utc::now() - t!($amount seconds)
    }};
    ($amount:literal minutes ago) => {{
        chrono::Utc::now() - t!($amount minutes)
    }};
    ($amount:literal hours ago) => {{
        chrono::Utc::now() - t!($amount hours)
    }};
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Timelike, Utc};

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
