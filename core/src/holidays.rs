//! Russian work calendar: public holidays and government-decreed workday
//! transfers, compiled in from `holidays_ru.json` (see `build.rs`).
//!
//! `timelog` uses [`HolidayCalendar::is_workday`] so a holiday weekday carries
//! no plan and a Saturday declared a workday does. The dataset holds calendar
//! facts (RF decrees), which are not copyrightable. Selecting a calendar by
//! locale or per server is an open question tracked in `TODO.md`; for now the RF
//! calendar is the single default.

use chrono::{Datelike, NaiveDate, Weekday};
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/holidays_data.rs"));

/// Russian holiday and workday calendar built from compile-time data.
///
/// Backed by two sorted `Vec<NaiveDate>` (holidays, transferred workdays) with
/// binary-search lookup.
#[derive(Debug)]
pub struct HolidayCalendar {
    holidays: Vec<NaiveDate>,
    workdays: Vec<NaiveDate>,
}

impl HolidayCalendar {
    /// The process-wide singleton calendar. Initialized once.
    pub fn global() -> &'static HolidayCalendar {
        static CALENDAR: OnceLock<HolidayCalendar> = OnceLock::new();
        CALENDAR.get_or_init(|| HolidayCalendar::from_static(HOLIDAYS, WORKDAYS))
    }

    /// Build from `(year, month, day)` triples, sorting and deduplicating so
    /// [`is_workday`](Self::is_workday) can binary-search.
    fn from_static(holidays: &[(i32, u32, u32)], workdays: &[(i32, u32, u32)]) -> Self {
        let to_sorted = |src: &[(i32, u32, u32)]| {
            let mut v: Vec<NaiveDate> = src
                .iter()
                .filter_map(|&(y, m, d)| NaiveDate::from_ymd_opt(y, m, d))
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        Self {
            holidays: to_sorted(holidays),
            workdays: to_sorted(workdays),
        }
    }

    /// Whether `date` is a workday under the Russian calendar.
    ///
    /// A decreed workday transfer (a weekend day declared working) wins over the
    /// weekday check; a listed public holiday is not a workday; otherwise it is a
    /// workday iff it is Mon-Fri. Dates outside the bundled year coverage fall
    /// back to the plain weekday rule.
    pub fn is_workday(&self, date: NaiveDate) -> bool {
        if self.workdays.binary_search(&date).is_ok() {
            return true;
        }
        if self.holidays.binary_search(&date).is_ok() {
            return false;
        }
        !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn regular_weekday_and_weekend() {
        let cal = HolidayCalendar::global();
        assert!(cal.is_workday(d(2026, 7, 8))); // Wednesday
        assert!(!cal.is_workday(d(2026, 7, 11))); // Saturday
        assert!(!cal.is_workday(d(2026, 7, 12))); // Sunday
    }

    #[test]
    fn new_year_holidays_are_not_workdays() {
        let cal = HolidayCalendar::global();
        for day in 1..=8 {
            assert!(!cal.is_workday(d(2026, 1, day)), "2026-01-{day:02}");
        }
        assert!(cal.is_workday(d(2026, 1, 12))); // first workday after the break
    }

    #[test]
    fn transferred_holiday_weekday_is_not_a_workday() {
        // 2026-03-09 (Monday) is a decreed holiday (Women's Day transfer).
        assert!(!HolidayCalendar::global().is_workday(d(2026, 3, 9)));
    }

    #[test]
    fn decreed_workday_on_weekend_is_a_workday() {
        // No transfer Saturdays in the shipped dataset; cover the branch with a
        // hand-built calendar: a Saturday declared a working day.
        let saturday = d(2025, 11, 1); // Saturday
        let cal = HolidayCalendar::from_static(&[], &[(2025, 11, 1)]);
        assert!(cal.is_workday(saturday));
    }

    #[test]
    fn calendar_is_sorted_and_unique() {
        let cal = HolidayCalendar::global();
        assert!(cal.holidays.windows(2).all(|w| w[0] < w[1]));
        assert!(cal.workdays.windows(2).all(|w| w[0] < w[1]));
        assert!(!cal.holidays.is_empty());
    }

    /// The build pipeline must round-trip `holidays_ru.json` into the compiled
    /// `HOLIDAYS`/`WORKDAYS` arrays (catches silent drops or schema drift).
    #[test]
    fn build_pipeline_matches_json_source() {
        let raw = include_str!("../holidays_ru.json");
        let parsed: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
        let root = parsed.as_object().expect("top-level object");

        let collect = |key: &str| {
            let mut out: Vec<(i32, u32, u32)> = Vec::new();
            for (_year, year_data) in root {
                if let Some(arr) = year_data.get(key).and_then(|v| v.as_array()) {
                    for entry in arr {
                        let s = entry.as_str().expect("date string");
                        let nd = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                            .unwrap_or_else(|e| panic!("bad date {s:?}: {e}"));
                        out.push((nd.year(), nd.month(), nd.day()));
                    }
                }
            }
            out.sort_unstable();
            out.dedup();
            out
        };

        let mut compiled_h = HOLIDAYS.to_vec();
        compiled_h.sort_unstable();
        compiled_h.dedup();
        let mut compiled_w = WORKDAYS.to_vec();
        compiled_w.sort_unstable();
        compiled_w.dedup();

        assert_eq!(compiled_h, collect("holidays"));
        assert_eq!(compiled_w, collect("workdays"));
    }
}
