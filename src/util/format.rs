use chrono::{DateTime, Utc};

const SECS_PER_MINUTE: i64 = 60;
const MINS_PER_HOUR: i64 = 60;
const HOURS_PER_DAY: i64 = 24;
const DAYS_PER_MONTH: i64 = 30;
const DAYS_PER_YEAR: i64 = 365;
const THOUSAND: u64 = 1_000;
const MILLION: u64 = 1_000_000;

/// Format a datetime string as relative time (e.g., "2h ago", "3d ago", "1mo ago")
pub fn relative_time(date_str: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(date_str) else {
        return date_str.to_string();
    };
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < SECS_PER_MINUTE {
        "just now".to_string()
    } else if duration.num_minutes() < MINS_PER_HOUR {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < HOURS_PER_DAY {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < DAYS_PER_MONTH {
        format!("{}d ago", duration.num_days())
    } else if duration.num_days() < DAYS_PER_YEAR {
        format!("{}mo ago", duration.num_days() / DAYS_PER_MONTH)
    } else {
        format!("{}y ago", duration.num_days() / DAYS_PER_YEAR)
    }
}

/// Abbreviate large numbers (e.g., 1200 -> "1.2k")
pub fn abbreviate_number(n: u64) -> String {
    if n >= MILLION {
        format!("{:.1}M", n as f64 / MILLION as f64)
    } else if n >= THOUSAND {
        format!("{:.1}k", n as f64 / THOUSAND as f64)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn abbreviate_small_numbers() {
        assert_eq!(abbreviate_number(0), "0");
        assert_eq!(abbreviate_number(1), "1");
        assert_eq!(abbreviate_number(999), "999");
    }

    #[test]
    fn abbreviate_thousands() {
        assert_eq!(abbreviate_number(1000), "1.0k");
        assert_eq!(abbreviate_number(1200), "1.2k");
        assert_eq!(abbreviate_number(15500), "15.5k");
    }

    #[test]
    fn abbreviate_millions() {
        assert_eq!(abbreviate_number(1_000_000), "1.0M");
        assert_eq!(abbreviate_number(2_500_000), "2.5M");
    }

    #[test]
    fn relative_time_invalid_input() {
        assert_eq!(relative_time("not-a-date"), "not-a-date");
        assert_eq!(relative_time(""), "");
    }

    #[test]
    fn relative_time_format_produces_suffix() {
        let result = relative_time("2020-01-01T00:00:00Z");
        assert!(
            result.ends_with("y ago") || result.ends_with("mo ago") || result.ends_with("d ago"),
            "unexpected format: {}",
            result
        );
    }

    #[test]
    fn relative_time_45_days_shows_months() {
        // 45 days > DAYS_PER_MONTH (30), should show "1mo ago"
        let now = Utc::now();
        let date = now - chrono::Duration::days(45);
        let date_str = date.to_rfc3339();
        let result = relative_time(&date_str);
        assert!(result.ends_with("mo ago"), "45 days should be months, got: {}", result);
    }

    #[test]
    fn relative_time_59_seconds_shows_just_now() {
        let now = Utc::now();
        let date = now - chrono::Duration::seconds(59);
        let date_str = date.to_rfc3339();
        let result = relative_time(&date_str);
        assert_eq!(result, "just now");
    }

    #[test]
    fn relative_time_60_seconds_shows_minutes() {
        let now = Utc::now();
        let date = now - chrono::Duration::seconds(60);
        let date_str = date.to_rfc3339();
        let result = relative_time(&date_str);
        assert!(result.ends_with("m ago"), "60 seconds should be minutes, got: {}", result);
    }
}
