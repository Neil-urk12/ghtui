use chrono::{DateTime, Utc};

/// Format a datetime string as relative time (e.g., "2h ago", "3d ago", "1mo ago")
pub fn relative_time(date_str: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(date_str) else {
        return date_str.to_string();
    };
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_days() < 365 {
        format!("{}mo ago", duration.num_days() / 30)
    } else {
        format!("{}y ago", duration.num_days() / 365)
    }
}

/// Abbreviate large numbers (e.g., 1200 -> "1.2k")
pub fn abbreviate_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
