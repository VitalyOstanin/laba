//! Conversions between fractional hours and ISO 8601 durations.
//!
//! Mirrors the semantics of the legacy Python `duration.py` helper used by the
//! time-entry resource.

use crate::error::Error;

/// Format fractional hours as an ISO 8601 duration string (e.g. `PT1H30M`).
///
/// Negative input is rejected with [`Error::Usage`]. Hours are rounded to whole
/// minutes before formatting.
pub fn hours_to_iso8601(hours: f64) -> Result<String, Error> {
    if hours < 0.0 {
        return Err(Error::Usage(format!("hours cannot be negative: {hours}")));
    }

    let total_minutes = (hours * 60.0).round() as i64;
    let whole_hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    let mut out = String::from("PT");
    if whole_hours > 0 {
        out.push_str(&format!("{whole_hours}H"));
    }
    if minutes > 0 || whole_hours == 0 {
        out.push_str(&format!("{minutes}M"));
    }
    Ok(out)
}

/// Parse an ISO 8601 duration string into fractional hours.
///
/// Accepts the pattern `P(<days>D)?(T(<hours>H)?(<minutes>M)?(<seconds>S)?)?`
/// where the numeric fields are decimal. Returns [`None`] for an empty string or
/// any value that does not match the pattern.
pub fn iso8601_to_hours(value: &str) -> Option<f64> {
    if value.is_empty() {
        return None;
    }

    let mut rest = value.strip_prefix('P')?;

    let mut days = 0.0;
    let mut hours = 0.0;
    let mut minutes = 0.0;
    let mut seconds = 0.0;

    // Optional days component, before the time designator.
    if let Some((num, tail)) = take_number(rest) {
        let tail = tail.strip_prefix('D')?;
        days = num;
        rest = tail;
    }

    // Optional time part, introduced by `T`.
    if let Some(time) = rest.strip_prefix('T') {
        let mut time_rest = time;
        // Each field must appear at most once and in H, M, S order.
        let mut expect: &[char] = &['H', 'M', 'S'];
        while !time_rest.is_empty() {
            let (num, tail) = take_number(time_rest)?;
            let unit = tail.chars().next()?;
            let pos = expect.iter().position(|&c| c == unit)?;
            match unit {
                'H' => hours = num,
                'M' => minutes = num,
                'S' => seconds = num,
                _ => return None,
            }
            expect = &expect[pos + 1..];
            time_rest = &tail[1..];
        }
    } else if !rest.is_empty() {
        // Trailing characters after the days component but no time designator.
        return None;
    }

    Some(days * 24.0 + hours + minutes / 60.0 + seconds / 3600.0)
}

/// Parse a human duration ("1h30m", "90m", "1.5h", "2h", "45m") into fractional hours.
/// At least one of the hours/minutes components is required; a bare number without
/// an h/m suffix is rejected (use decimal hours via the --hours flag instead).
/// Case-insensitive on the suffixes.
pub fn parse_human_duration(s: &str) -> Result<f64, Error> {
    let err = || {
        Error::Usage(format!(
            "invalid duration: {s:?} (expected e.g. 1h30m, 90m, 1.5h)"
        ))
    };
    // Whitespace between components is allowed and ignored ("5h 12m", "1h 30m").
    let lower: String = s
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut rest = lower.as_str();
    let mut hours = 0.0;
    let mut minutes = 0.0;
    let mut any = false;

    if let Some((num, tail)) = take_number(rest) {
        if let Some(after) = tail.strip_prefix('h') {
            hours = num;
            rest = after;
            any = true;
        }
    }
    if let Some((num, tail)) = take_number(rest) {
        if let Some(after) = tail.strip_prefix('m') {
            minutes = num;
            rest = after;
            any = true;
        }
    }

    if !any || !rest.is_empty() {
        return Err(err());
    }
    Ok(hours + minutes / 60.0)
}

/// Consume a leading decimal number (`\d+(\.\d+)?`) from `s`.
///
/// Returns the parsed value and the remaining string, or [`None`] if `s` does
/// not start with a digit or the numeric literal is malformed.
fn take_number(s: &str) -> Option<(f64, &str)> {
    let mut end = 0;
    let bytes = s.as_bytes();

    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        let mut frac = end + 1;
        while frac < bytes.len() && bytes[frac].is_ascii_digit() {
            frac += 1;
        }
        // A dot must be followed by at least one digit.
        if frac == end + 1 {
            return None;
        }
        end = frac;
    }

    let num = s[..end].parse().ok()?;
    Some((num, &s[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn hours_to_iso8601_examples() {
        assert_eq!(hours_to_iso8601(1.5).unwrap(), "PT1H30M");
        assert_eq!(hours_to_iso8601(0.0).unwrap(), "PT0M");
        assert_eq!(hours_to_iso8601(2.0).unwrap(), "PT2H");
        assert_eq!(hours_to_iso8601(0.25).unwrap(), "PT15M");
    }

    #[test]
    fn hours_to_iso8601_rejects_negative() {
        assert!(hours_to_iso8601(-1.0).is_err());
    }

    #[test]
    fn iso8601_to_hours_examples() {
        assert!(approx(iso8601_to_hours("PT1H30M").unwrap(), 1.5));
        assert!(approx(iso8601_to_hours("PT0M").unwrap(), 0.0));
        assert!(approx(iso8601_to_hours("P1DT2H").unwrap(), 26.0));
        assert_eq!(iso8601_to_hours(""), None);
        assert_eq!(iso8601_to_hours("garbage"), None);
    }

    #[test]
    fn parse_human_duration_examples() {
        assert!(approx(parse_human_duration("1h30m").unwrap(), 1.5));
        assert!(approx(parse_human_duration("90m").unwrap(), 1.5));
        assert!(approx(parse_human_duration("1.5h").unwrap(), 1.5));
        assert!(approx(parse_human_duration("2h").unwrap(), 2.0));
        assert!(approx(parse_human_duration("45m").unwrap(), 0.75));
        assert!(approx(
            parse_human_duration("5h 12m").unwrap(),
            5.0 + 12.0 / 60.0
        ));
        assert!(approx(parse_human_duration("1h 30m").unwrap(), 1.5));
        assert!(approx(parse_human_duration(" 90m ").unwrap(), 1.5));
        assert!(approx(parse_human_duration("1H30M").unwrap(), 1.5));
    }

    #[test]
    fn parse_human_duration_rejects_invalid() {
        for s in ["90", "1h30", "", "abc", "h", "m", "1m30h"] {
            assert!(parse_human_duration(s).is_err(), "expected error for {s:?}");
        }
    }
}
