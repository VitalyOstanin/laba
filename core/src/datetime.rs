//! Configurable display / day-boundary timezone.
//!
//! One zone drives both the timelog "today" boundary and the rendering of API
//! datetimes, shared by the CLI and GUI. It defaults to the machine's local zone
//! and can be overridden with an IANA name (e.g. `Europe/Moscow`). An unparseable
//! name falls back to local so a bad setting never breaks a run; validate with
//! [`Zone::parse`] at the point a user enters it.

use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;

use crate::error::Error;

/// A resolved timezone: an explicit IANA zone, or the machine's local zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone(Option<Tz>);

impl Zone {
    /// The machine's local zone.
    pub fn local() -> Zone {
        Zone(None)
    }

    /// An explicit IANA zone.
    pub fn named(tz: Tz) -> Zone {
        Zone(Some(tz))
    }

    /// Resolve an optional IANA name leniently: `None`, empty, or an unknown name
    /// yields the local zone. Use on the runtime path where a bad value must not
    /// fail the operation.
    pub fn resolve(name: Option<&str>) -> Zone {
        match name {
            Some(n) if !n.is_empty() => n
                .parse::<Tz>()
                .map(Zone::named)
                .unwrap_or_else(|_| Zone::local()),
            _ => Zone::local(),
        }
    }

    /// Parse an IANA name strictly, returning a `Config` error on an unknown zone.
    /// Use to validate user input before persisting it.
    pub fn parse(name: &str) -> Result<Zone, Error> {
        name.parse::<Tz>()
            .map(Zone::named)
            .map_err(|_| Error::Config(format!("unknown timezone '{name}'")))
    }

    /// Today's calendar date in this zone.
    pub fn today(&self) -> NaiveDate {
        match self.0 {
            Some(tz) => chrono::Utc::now().with_timezone(&tz).date_naive(),
            None => chrono::Local::now().date_naive(),
        }
    }

    /// Render an RFC 3339 / ISO 8601 datetime as `YYYY-MM-DD HH:MM` in this zone.
    /// Returns the input unchanged when it does not parse, so callers can format
    /// opaquely without pre-validating.
    pub fn format_datetime(&self, iso: &str) -> String {
        let dt = match DateTime::parse_from_rfc3339(iso) {
            Ok(d) => d,
            Err(_) => return iso.to_string(),
        };
        match self.0 {
            Some(tz) => dt.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string(),
            None => dt
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_is_lenient() {
        assert_eq!(Zone::resolve(None), Zone::local());
        assert_eq!(Zone::resolve(Some("")), Zone::local());
        assert_eq!(Zone::resolve(Some("Not/AZone")), Zone::local());
        assert_eq!(
            Zone::resolve(Some("Europe/Moscow")),
            Zone::named(Tz::Europe__Moscow)
        );
    }

    #[test]
    fn parse_is_strict() {
        assert!(Zone::parse("Europe/Moscow").is_ok());
        assert!(matches!(Zone::parse("nope"), Err(Error::Config(_))));
    }

    #[test]
    fn format_datetime_converts_to_named_zone() {
        let z = Zone::named(Tz::Europe__Moscow); // UTC+3, no DST
                                                 // 09:30 UTC -> 12:30 Moscow.
        assert_eq!(
            z.format_datetime("2026-07-08T09:30:00Z"),
            "2026-07-08 12:30"
        );
        // An offset input is normalized to the zone: 12:30+03:00 == 09:30 UTC.
        assert_eq!(
            z.format_datetime("2026-07-08T12:30:00+03:00"),
            "2026-07-08 12:30"
        );
    }

    #[test]
    fn format_datetime_passes_through_unparseable() {
        let z = Zone::local();
        assert_eq!(z.format_datetime("not-a-date"), "not-a-date");
        assert_eq!(z.format_datetime(""), "");
    }

    #[test]
    fn today_in_named_zone_is_a_valid_date() {
        // Just assert it resolves without panicking and differs across far-apart
        // zones is not guaranteed at all times, so only check it returns.
        let _ = Zone::named(Tz::Europe__Moscow).today();
        let _ = Zone::local().today();
    }
}
