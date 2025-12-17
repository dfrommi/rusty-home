use crate::core::time::DateTime;

#[derive(Debug, Clone)]
pub struct Metric {
    pub id: MetricId,
    pub value: f64,
    pub timestamp: DateTime,
}

#[derive(Debug, Clone)]
pub struct MetricId {
    pub name: String,
    pub labels: Vec<MetricLabel>,
}

#[derive(Debug, Clone)]
pub enum MetricLabel {
    Variant(String),
    Room(String),
    FriendlyName(String),
    EnumVariant(String),
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.id, self.value, self.timestamp.millis())
    }
}

impl std::fmt::Display for MetricId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels: Vec<String> = self.labels.iter().map(|label| label.to_string()).collect();
        if labels.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}{{{}}}", self.name, labels.join(", "))
        }
    }
}

impl std::fmt::Display for MetricLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricLabel::Variant(v) => write!(f, "item=\"{}\"", v),
            MetricLabel::Room(r) => write!(f, "room=\"{}\"", r),
            MetricLabel::FriendlyName(n) => write!(f, "friendly_name=\"{}\"", n),
            MetricLabel::EnumVariant(ev) => write!(f, "enum_variant=\"{}\"", ev),
        }
    }
}
