use super::DateTime;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Duration {
    #[serde(with = "duration_format")]
    pub(super) delegate: chrono::Duration,
}

impl Duration {
    pub(super) fn new(delegate: chrono::Duration) -> Self {
        Self { delegate }
    }

    pub fn zero() -> Self {
        Self::new(chrono::Duration::zero())
    }

    pub fn until(date_time: &DateTime) -> Self {
        Self::new(*date_time.delegate() - DateTime::now().delegate())
    }

    pub fn days(days: i64) -> Self {
        Self::new(chrono::Duration::days(days))
    }

    pub fn hours(hours: i64) -> Self {
        Self::new(chrono::Duration::hours(hours))
    }

    pub fn minutes(minutes: i64) -> Self {
        Self::new(chrono::Duration::minutes(minutes))
    }

    pub fn seconds(seconds: i64) -> Self {
        Self::new(chrono::Duration::seconds(seconds))
    }

    pub fn millis(millis: i64) -> Self {
        Self::new(chrono::Duration::milliseconds(millis))
    }

    pub fn as_secs(&self) -> i64 {
        self.delegate.num_seconds()
    }

    pub fn as_secs_f64(&self) -> f64 {
        self.delegate.num_milliseconds() as f64 / 1000.0
    }

    pub fn as_hours_f64(&self) -> f64 {
        self.delegate.num_milliseconds() as f64 / 1000.0 / 3600.0
    }

    pub fn as_minutes(&self) -> i64 {
        self.delegate.num_minutes()
    }

    pub fn as_hours(&self) -> i64 {
        self.delegate.num_hours()
    }

    pub fn to_iso_string(&self) -> String {
        from_chrono_duration(&self.delegate).to_string()
    }

    pub fn into_db(&self) -> chrono::Duration {
        self.delegate
    }
}

impl std::ops::Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            delegate: self.delegate + rhs.delegate,
        }
    }
}

impl From<Duration> for std::time::Duration {
    fn from(val: Duration) -> Self {
        let millis = val.delegate.num_milliseconds();
        std::time::Duration::from_millis(millis as u64)
    }
}

mod duration_format {
    use iso8601_duration::Duration as Iso8601Duration;
    use serde::{de::Visitor, Deserializer, Serializer};

    // Serialize `chrono::Duration` to ISO 8601 string format (e.g., "P1DT2H30M")
    pub fn serialize<S>(duration: &chrono::TimeDelta, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let iso_duration = super::from_chrono_duration(duration);
        let iso_string = iso_duration.to_string();
        serializer.serialize_str(&iso_string)
    }

    // Deserialize ISO 8601 string format to `chrono::Duration`
    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl Visitor<'_> for DurationVisitor {
            type Value = chrono::TimeDelta;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a string representing an ISO 8601 duration (e.g., P1DT2H30M)")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Parse the ISO 8601 string into a `iso8601-duration::Duration`
                let iso_duration = Iso8601Duration::parse(value).map_err(|e| {
                    E::custom(format!("Error parsing {} to duration: {:?}", value, e))
                })?;

                match iso_duration.to_chrono() {
                    Some(duration) => Ok(duration),
                    None => Err(E::custom(format!(
                        "Duration too long. Must not contain years and/or months. Received {}",
                        value
                    ))),
                }
            }
        }

        deserializer.deserialize_str(DurationVisitor)
    }
}

fn from_chrono_duration(duration: &chrono::Duration) -> iso8601_duration::Duration {
    let days = duration.num_days();
    let seconds = duration.num_seconds() - days * 86400; // remove days in seconds
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    iso8601_duration::Duration::new(
        0.0, //years
        0.0, //months
        days as f32,
        hours as f32,
        minutes as f32,
        seconds as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t;

    #[test]
    fn test_serialize_duration() {
        let duration = t!(8 hours) + t!(15 minutes);
        let serialized = serde_json::to_string(&duration).unwrap();
        assert_eq!(serialized, r#""PT8H15M""#);
    }

    #[test]
    fn test_deserialize_duration() {
        let duration = serde_json::from_str::<Duration>(r#""PT8H15M""#).unwrap();
        assert_eq!(duration, t!(8 hours) + t!(15 minutes));
    }
}
