//! Duration parsing utilities for timeout and wait commands.

use anyhow::{Result, bail};
use std::time::Duration;

/// Parse a duration string like "30s", "5m", "1h", "10m30s".
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            let n: u64 = current_num
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid duration: '{}'", s))?;
            current_num.clear();
            match ch {
                's' => total_secs += n,
                'm' => total_secs += n * 60,
                'h' => total_secs += n * 3600,
                'd' => total_secs += n * 86400,
                _ => bail!("Invalid duration unit '{}' in '{}'", ch, s),
            }
        }
    }

    // If there's a trailing number with no unit, treat as seconds
    if !current_num.is_empty() {
        let n: u64 = current_num
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid duration: '{}'", s))?;
        total_secs += n;
    }

    if total_secs == 0 {
        bail!("Duration must be greater than zero: '{}'", s);
    }

    Ok(Duration::from_secs(total_secs))
}

#[cfg(test)]
#[path = "duration_tests.rs"]
mod tests;
