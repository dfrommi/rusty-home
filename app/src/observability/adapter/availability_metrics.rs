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
                    labels: labels.clone(),
                },
                value: if status.is_offline { 1.0 } else { 0.0 },
                timestamp,
            },
            Metric {
                id: MetricId {
                    name: "device_disabled".to_string(),
                    labels,
                },
                value: if status.disabled { 1.0 } else { 0.0 },
                timestamp,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::{device_state::DeviceAvailabilityStatus, t};

    use super::*;

    #[test]
    fn exports_disabled_state_as_a_stable_metric() {
        let metrics = AvailabilityMetricsAdapter.to_metrics(DeviceAvailabilityStatus {
            source: "Tasmota".to_string(),
            item: "removed".to_string(),
            last_seen_ago: t!(1 hours),
            is_offline: true,
            disabled: true,
        });

        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].id.name, "device_last_seen_seconds");
        assert_eq!(metrics[1].id.name, "device_offline");
        assert_eq!(metrics[2].id.name, "device_disabled");
        assert!(metrics.iter().all(|metric| metric.id.labels
            == vec![
                MetricLabel::Variant("removed".to_string()),
                MetricLabel::Source("Tasmota".to_string()),
            ]));
        assert_eq!(metrics[2].value, 1.0);
    }
}
