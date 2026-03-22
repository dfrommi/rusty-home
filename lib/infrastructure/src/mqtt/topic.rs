pub(super) fn join_topic(base_topic: impl AsRef<str>, topic: impl AsRef<str>) -> String {
    let base_topic = base_topic.as_ref().trim_matches('/');
    let topic = topic.as_ref().trim_matches('/');

    match (base_topic.is_empty(), topic.is_empty()) {
        (true, true) => String::new(),
        (false, true) => base_topic.to_string(),
        (true, false) => topic.to_string(),
        (false, false) => format!("{base_topic}/{topic}"),
    }
}

pub(super) fn strip_topic<'a>(base_topic: &str, full_topic: &'a str) -> Option<&'a str> {
    let base = base_topic.trim_matches('/');
    let full = full_topic.trim_matches('/');

    if base.is_empty() {
        return Some(full);
    }

    if full == base {
        return Some("");
    }

    full.strip_prefix(base).and_then(|rest| rest.strip_prefix('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_topic_uses_single_separator() {
        assert_eq!(join_topic("home/base/", "/device/set"), "home/base/device/set");
    }

    #[test]
    fn join_topic_handles_missing_separators() {
        assert_eq!(join_topic("home/base", "device/set"), "home/base/device/set");
    }

    #[test]
    fn join_topic_handles_empty_base() {
        assert_eq!(join_topic("", "/device/set/"), "device/set");
    }

    #[test]
    fn strip_topic_removes_base() {
        assert_eq!(strip_topic("zigbee", "zigbee/device_name"), Some("device_name"));
    }

    #[test]
    fn strip_topic_removes_base_with_slashes() {
        assert_eq!(strip_topic("zigbee/", "/zigbee/device_name/"), Some("device_name"));
    }

    #[test]
    fn strip_topic_preserves_nested_path() {
        assert_eq!(strip_topic("tasmota", "tasmota/tele/device/SENSOR"), Some("tele/device/SENSOR"));
    }

    #[test]
    fn strip_topic_exact_match_returns_empty() {
        assert_eq!(strip_topic("homeassistant/events", "homeassistant/events"), Some(""));
    }

    #[test]
    fn strip_topic_empty_base_returns_full() {
        assert_eq!(strip_topic("", "some/topic"), Some("some/topic"));
    }

    #[test]
    fn strip_topic_non_matching_returns_none() {
        assert_eq!(strip_topic("zigbee", "tasmota/device"), None);
    }

    #[test]
    fn strip_topic_partial_prefix_match_returns_none() {
        assert_eq!(strip_topic("zig", "zigbee/device"), None);
    }
}
