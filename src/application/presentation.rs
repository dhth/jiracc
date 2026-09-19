use chrono::{DateTime, Utc};

pub(super) fn format_relative_time(
    timestamp: DateTime<Utc>,
    reference_time: DateTime<Utc>,
) -> Option<String> {
    if reference_time < timestamp {
        return None;
    }

    let elapsed_seconds = reference_time
        .signed_duration_since(timestamp)
        .num_seconds();

    let relative_time = if elapsed_seconds < 60 {
        "just now".to_owned()
    } else if elapsed_seconds < 60 * 60 {
        format!("{}m ago", elapsed_seconds / 60)
    } else if elapsed_seconds < 24 * 60 * 60 {
        let hours = elapsed_seconds / (60 * 60);
        let minutes = (elapsed_seconds % (60 * 60)) / 60;

        if minutes == 0 {
            format!("{hours}h ago")
        } else {
            format!("{hours}h {minutes}m ago")
        }
    } else {
        let days = elapsed_seconds / (24 * 60 * 60);
        let hours = (elapsed_seconds % (24 * 60 * 60)) / (60 * 60);

        if hours == 0 {
            format!("{days}d ago")
        } else {
            format!("{days}d {hours}h ago")
        }
    };

    Some(relative_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    #[test]
    fn format_relative_time_handles_elapsed_time_boundaries() -> anyhow::Result<()> {
        let reference_time = "2026-09-18T15:15:00Z".parse::<DateTime<Utc>>()?;
        let cases = [
            (TimeDelta::milliseconds(-1), None),
            (TimeDelta::zero(), Some("just now")),
            (TimeDelta::seconds(59), Some("just now")),
            (TimeDelta::seconds(60), Some("1m ago")),
            (TimeDelta::seconds(24 * 60 + 59), Some("24m ago")),
            (TimeDelta::seconds(3_599), Some("59m ago")),
            (TimeDelta::seconds(3_600), Some("1h ago")),
            (
                TimeDelta::seconds(13 * 3_600 + 45 * 60 + 59),
                Some("13h 45m ago"),
            ),
            (TimeDelta::seconds(86_399), Some("23h 59m ago")),
            (TimeDelta::seconds(86_400), Some("1d ago")),
            (TimeDelta::seconds(4 * 86_400), Some("4d ago")),
            (
                TimeDelta::seconds(4 * 86_400 + 14 * 3_600 + 3_599),
                Some("4d 14h ago"),
            ),
        ];

        for (elapsed, expected) in cases {
            let timestamp = reference_time - elapsed;

            assert_eq!(
                format_relative_time(timestamp, reference_time).as_deref(),
                expected
            );
        }

        Ok(())
    }
}
