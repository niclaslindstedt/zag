use super::*;

#[test]
fn test_shell_escape_simple() {
    assert_eq!(shell_escape("hello"), "'hello'");
}

#[test]
fn test_shell_escape_spaces() {
    assert_eq!(shell_escape("hello world"), "'hello world'");
}

#[test]
fn test_shell_escape_single_quotes() {
    assert_eq!(shell_escape("it's"), "\"it's\"");
}

#[test]
fn test_shell_escape_double_quotes() {
    assert_eq!(shell_escape("say \"hi\""), "'say \"hi\"'");
}
