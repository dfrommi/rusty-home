#[macro_export]
macro_rules! t {
    (now) => {{
        $crate::core::time::DateTime::now()
    }};

    ($from_hour:literal : $from_minute:literal - $to_hour:literal : $to_minute:literal) => {{
        $crate::core::time::DailyTimeRange::new(t!($from_hour:$from_minute), t!($to_hour:$to_minute))
    }};

    ($hour:literal : $minute:literal) => {{
        $crate::core::time::Time::at($hour, $minute).unwrap()
    }};

    ($amount:literal seconds) => {{
        $crate::core::time::Duration::seconds($amount)
    }};
    ($amount:literal minutes) => {{
        $crate::core::time::Duration::minutes($amount)
    }};
    ($amount:literal hours) => {{
        $crate::core::time::Duration::hours($amount)
    }};

    ($amount:literal seconds ago) => {{
        t!(now) - t!($amount seconds)
    }};
    ($amount:literal minutes ago) => {{
        t!(now) - t!($amount minutes)
    }};
    ($amount:literal hours ago) => {{
        t!(now) - t!($amount hours)
    }};

    (in $amount:literal seconds) => {{
        t!(now) + t!($amount seconds)
    }};
    (in $amount:literal minutes) => {{
        t!(now) + t!($amount minutes)
    }};
    (in $amount:literal hours) => {{
        t!(now) + t!($amount hours)
    }};
}

#[cfg(test)]
mod tests {
    use crate::core::time::*;

    #[test]
    fn test_now() {
        let now = t!(now);
        assert!(DateTime::now().elapsed_since(now) < Duration::seconds(1));
    }

    #[test]
    fn test_time() {
        let t = t!(5:34);

        assert_eq!(t.hour(), 5);
        assert_eq!(t.minute(), 34);
    }

    #[test]
    fn test_duration_seconds() {
        let duration = t!(10 seconds);

        assert_eq!(duration.as_secs(), 10);
    }

    #[test]
    fn test_duration_minutes() {
        let duration = t!(10 minutes);

        assert_eq!(duration.as_minutes(), 10);
    }

    #[test]
    fn test_duration_hours() {
        let duration = t!(10 hours);

        assert_eq!(duration.as_hours(), 10);
    }

    #[test]
    fn test_duration_seconds_ago() {
        let now = DateTime::now();
        let dt = t!(10 seconds ago);

        assert!(now >= dt);
        assert!(now - Duration::seconds(10) <= dt);
    }

    #[test]
    fn test_duration_minutes_ago() {
        let now = DateTime::now();
        let dt = t!(10 minutes ago);

        assert!(now >= dt);
        assert!(now - Duration::minutes(10) <= dt);
    }

    #[test]
    fn test_duration_hours_ago() {
        let now = DateTime::now();
        let dt = t!(10 hours ago);

        assert!(now >= dt);
        assert!(now - Duration::hours(10) <= dt);
    }
}
