use chrono::{Local, NaiveDate, TimeDelta, Utc};

/// Parse a time expression into a Slack timestamp string (unix epoch with `.000000` suffix).
///
/// Accepted formats:
/// - `today` — start of today (local time)
/// - Relative: `30m`, `1h`, `2d` (minutes, hours, days ago from now)
/// - ISO 8601: `2024-01-15T10:30:00Z` or `2024-01-15`
pub fn parse_time_expr(s: &str) -> Result<String, String> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("today") {
        let today = Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .ok_or("failed to compute start of today")?;
        let ts = today
            .and_local_timezone(Local)
            .single()
            .ok_or("ambiguous local time")?
            .timestamp();
        return Ok(format!("{}.000000", ts));
    }

    // Relative: 30m, 1h, 2d
    if let Some(val) = parse_relative(s) {
        let now = Utc::now();
        let then = now - val;
        return Ok(format!("{}.000000", then.timestamp()));
    }

    // ISO datetime
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(format!("{}.000000", dt.timestamp()));
    }

    // ISO date only (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or("failed to build datetime from date")?;
        let ts = dt
            .and_local_timezone(Local)
            .single()
            .ok_or("ambiguous local time")?
            .timestamp();
        return Ok(format!("{}.000000", ts));
    }

    Err(format!(
        "Invalid time expression: '{}'. Use ISO timestamp, relative (1h/30m/2d), or 'today'.",
        s
    ))
}

fn parse_relative(s: &str) -> Option<TimeDelta> {
    let s = s.trim();
    if s.len() < 2 {
        return None;
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str.parse().ok()?;
    match unit {
        "m" => TimeDelta::try_minutes(num),
        "h" => TimeDelta::try_hours(num),
        "d" => TimeDelta::try_days(num),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_minutes() {
        let result = parse_time_expr("30m").unwrap();
        assert!(result.ends_with(".000000"));
        let ts: f64 = result.parse().unwrap();
        let now = Utc::now().timestamp() as f64;
        // Should be ~30 minutes ago (allow 5s tolerance)
        assert!((now - ts - 1800.0).abs() < 5.0);
    }

    #[test]
    fn test_relative_hours() {
        let result = parse_time_expr("2h").unwrap();
        let ts: f64 = result.parse().unwrap();
        let now = Utc::now().timestamp() as f64;
        assert!((now - ts - 7200.0).abs() < 5.0);
    }

    #[test]
    fn test_relative_days() {
        let result = parse_time_expr("1d").unwrap();
        let ts: f64 = result.parse().unwrap();
        let now = Utc::now().timestamp() as f64;
        assert!((now - ts - 86400.0).abs() < 5.0);
    }

    #[test]
    fn test_today() {
        let result = parse_time_expr("today").unwrap();
        assert!(result.ends_with(".000000"));
    }

    #[test]
    fn test_iso_datetime() {
        let result = parse_time_expr("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(result, "1705314600.000000");
    }

    #[test]
    fn test_iso_date() {
        let result = parse_time_expr("2024-01-15").unwrap();
        assert!(result.ends_with(".000000"));
    }

    #[test]
    fn test_invalid() {
        assert!(parse_time_expr("foobar").is_err());
    }
}
