use super::*;

#[test]
fn test_normalize_url_no_scheme() {
    assert_eq!(normalize_url("localhost:2100"), "https://localhost:2100");
}

#[test]
fn test_normalize_url_with_https() {
    assert_eq!(normalize_url("https://foo:2100"), "https://foo:2100");
}

#[test]
fn test_normalize_url_with_http() {
    assert_eq!(normalize_url("http://foo:2100"), "http://foo:2100");
}

#[test]
fn test_normalize_url_trailing_slash() {
    assert_eq!(normalize_url("foo:2100/"), "https://foo:2100");
}

#[test]
fn test_normalize_url_hostname_only() {
    assert_eq!(normalize_url("home.local:2100"), "https://home.local:2100");
}

#[test]
fn test_normalize_url_https_trailing_slash() {
    assert_eq!(normalize_url("https://foo:2100/"), "https://foo:2100");
}
