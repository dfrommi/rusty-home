use crate::{core::time::DateTime, device_state::DeviceAvailabilityStatus};

use super::{Metric, MetricId, MetricLabel, MetricsAdapter};

pub struct AvailabilityMetricsAdapter;

impl MetricsAdapter<DeviceAvailabilityStatus> for AvailabilityMetricsAdapter {
    fn to_metrics(&self, status: DeviceAvailabilityStatus) -> Vec<Metric> {
        let labels = vec![
            MetricLabel::Variant(status.item.clone()),
            MetricLabel::Source(status.source.clone()),
        ];

        let timestamp = DateTime::now();

        vec![
            Metric {
                id: MetricId {
                    name: "device_last_seen_seconds".to_string(),
                    labels: labels.clone(),
                },
                value: status.last_seen_ago.as_secs_f64(),
                timestamp,
            },
            Metric {
                id: MetricId {
                    name: "device_offline".to_string(),
                    labels,
                },
                value: if status.is_offline { 1.0 } else { 0.0 },
                timestamp,
            },
        ]
    }
}
