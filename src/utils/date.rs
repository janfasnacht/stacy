//! Date utilities for version fallback
//!
//! When SSC packages lack a `distribution_date`, we use today's date
//! in YYYYMMDD format (matching SSC's convention) as the version string.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns today's date as a "YYYYMMDD" string.
///
/// Uses Hinnant's civil calendar algorithm to convert Unix timestamp
/// to a calendar date without external dependencies.
pub fn today_yyyymmdd() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs();
    let (y, m, d) = civil_from_days((secs / 86400) as i64);
    format!("{:04}{:02}{:02}", y, m, d)
}

/// Hinnant's civil_from_days algorithm.
///
/// Converts a day count from the Unix epoch (1970-01-01) to (year, month, day).
/// See: <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month prime [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn test_known_date() {
        // 2025-01-25 is day 20113 from epoch
        // (2025-01-01 = day 20089, + 24 = 20113)
        assert_eq!(civil_from_days(20_113), (2025, 1, 25));
    }

    #[test]
    fn test_leap_year() {
        // 2024-02-29 (leap day)
        // 2024-01-01 = day 19723, + 59 = 19782
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }

    #[test]
    fn test_today_yyyymmdd_format() {
        let result = today_yyyymmdd();
        assert_eq!(result.len(), 8);
        assert!(result.chars().all(|c| c.is_ascii_digit()));

        // Year should be reasonable (2020-2099)
        let year: u32 = result[..4].parse().unwrap();
        assert!(year >= 2020 && year <= 2099);

        // Month 01-12
        let month: u32 = result[4..6].parse().unwrap();
        assert!((1..=12).contains(&month));

        // Day 01-31
        let day: u32 = result[6..8].parse().unwrap();
        assert!((1..=31).contains(&day));
    }
}
