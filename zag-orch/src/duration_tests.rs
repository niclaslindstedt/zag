use super::*;

#[test]
fn test_parse_duration_seconds() {
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
}

#[test]
fn test_parse_duration_minutes() {
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
    assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
}

#[test]
fn test_parse_duration_hours() {
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
}

#[test]
fn test_parse_duration_combined() {
    assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
    assert_eq!(parse_duration("1m30s").unwrap(), Duration::from_secs(90));
    assert_eq!(
        parse_duration("1h5m30s").unwrap(),
        Duration::from_secs(3930)
    );
}

#[test]
fn test_parse_duration_bare_number() {
    // Bare number treated as seconds
    assert_eq!(parse_duration("60").unwrap(), Duration::from_secs(60));
}

#[test]
fn test_parse_duration_invalid() {
    assert!(parse_duration("0s").is_err());
    assert!(parse_duration("abc").is_err());
    assert!(parse_duration("5x").is_err());
}

#[test]
fn test_parse_duration_days() {
    assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
}
