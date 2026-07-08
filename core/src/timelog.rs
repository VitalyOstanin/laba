//! Work-log status against an 8h/weekday plan.
//!
//! Pure computation shared by CLI and GUI. Ported from the predecessor
//! `openproject-gnome-notify` `lib/timelog.js` (`computeTimelogStatus`) with two
//! deliberate changes:
//!
//! - Aggregation is in whole minutes, not accumulated float hours, to avoid
//!   rounding drift (requirement 14).
//! - Surplus (logged above plan) is tracked and surfaced as a distinct status,
//!   so "exactly filled" is told apart from "overlogged" (requirement 25).
//!
//! Plan: 8h (480 min) per weekday, 0 on weekends, from a start date up to and
//! including today. Per-day shortfall is `max(0, plan - logged)`; overlog on one
//! day never offsets another day's shortfall. Fully-filled past weeks (zero
//! weekly shortfall) are dropped from the displayed plan/logged totals; the
//! current week is always kept.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Planned minutes for a full weekday.
pub const DAILY_NORM_MINUTES: i64 = 8 * 60;

/// `chrono` format string for the `YYYY-MM-DD` dates used throughout timelog.
const DATE_FMT: &str = "%Y-%m-%d";

/// A single day in the work-log timeline (requirement 16).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DayCell {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// A planned weekday (plan > 0) versus a weekend.
    pub weekday: bool,
    pub plan_min: i64,
    pub logged_min: i64,
    /// `max(0, plan - logged)`.
    pub deficit_min: i64,
    /// `max(0, logged - plan)`.
    pub surplus_min: i64,
}

/// Overall work-log status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// A shortfall on a past (already closed) day.
    Red,
    /// A shortfall only on today.
    Yellow,
    /// Everything met, including today.
    Green,
    /// Everything met and some day is overlogged (requirement 25).
    Over,
}

/// Aggregate result for the dashboard indicator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TimelogStatus {
    pub logged_min: i64,
    pub planned_min: i64,
    pub today_deficit_min: i64,
    /// Sum of per-day shortfalls over displayed weeks.
    pub deficit_min: i64,
    /// Sum of per-day surpluses over displayed weeks.
    pub surplus_min: i64,
    pub status: Status,
}

/// Convert normalized decimal hours to whole minutes (rounding to the nearest
/// minute), so aggregation stays integer (requirement 14).
pub fn minutes_from_hours(hours: f64) -> i64 {
    (hours * 60.0).round() as i64
}

/// Today's local date.
pub fn today_local() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Parse a `YYYY-MM-DD` date, returning `None` on any other shape.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, DATE_FMT).ok()
}

/// Format a date as `YYYY-MM-DD`.
pub fn fmt(date: NaiveDate) -> String {
    date.format(DATE_FMT).to_string()
}

/// Resolve the `[start, today]` window from a user-configured start date.
///
/// Returns `None` when no valid start date is configured: timelog is only
/// meaningful once the user sets a start (matching the predecessor, where an
/// empty start yields no deficit). There is no invented default.
pub fn window(start: Option<&str>) -> Option<(NaiveDate, NaiveDate)> {
    let start = parse_date(start?)?;
    Some((start, today_local()))
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

fn plan_minutes(date: NaiveDate) -> i64 {
    if is_weekend(date) {
        0
    } else {
        DAILY_NORM_MINUTES
    }
}

/// Local midnight of the Monday opening the week containing `date`.
fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// Sum logged minutes per `spentOn` day.
fn logged_by_day(entries: &[(String, i64)]) -> std::collections::HashMap<String, i64> {
    let mut map = std::collections::HashMap::new();
    for (day, minutes) in entries {
        if day.is_empty() {
            continue;
        }
        *map.entry(day.clone()).or_insert(0) += *minutes;
    }
    map
}

/// Build the full per-day timeline from `start` to `today` inclusive.
///
/// Weekends are included (with `weekday: false`) so the GUI can render or skip
/// them; they carry zero plan and therefore never contribute a deficit.
pub fn build_timeline(entries: &[(String, i64)], start: NaiveDate, today: NaiveDate) -> Vec<DayCell> {
    let by_day = logged_by_day(entries);
    let mut cells = Vec::new();
    let mut cur = start;
    while cur <= today {
        let key = cur.format(DATE_FMT).to_string();
        let plan = plan_minutes(cur);
        let logged = by_day.get(&key).copied().unwrap_or(0);
        cells.push(DayCell {
            date: key,
            weekday: plan > 0,
            plan_min: plan,
            logged_min: logged,
            deficit_min: (plan - logged).max(0),
            surplus_min: (logged - plan).max(0),
        });
        cur += Duration::days(1);
    }
    cells
}

/// A work package proposed for logging time, with how much is already logged in
/// the window (requirement 15). Ranked least-logged first so under-logged tasks
/// surface for filling gaps.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    pub server: String,
    pub wp_id: i64,
    pub subject: String,
    pub logged_min: i64,
}

/// Rank candidates by least logged time first, then by work package id for a
/// stable order.
pub fn rank_candidates(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by(|a, b| {
        a.logged_min
            .cmp(&b.logged_min)
            .then_with(|| a.wp_id.cmp(&b.wp_id))
    });
    candidates
}

/// An empty, all-green status (no window configured or no data).
pub fn empty_status() -> TimelogStatus {
    TimelogStatus {
        logged_min: 0,
        planned_min: 0,
        today_deficit_min: 0,
        deficit_min: 0,
        surplus_min: 0,
        status: Status::Green,
    }
}

/// Compute status and timeline from a string start date against today. Returns
/// `None` when `start` is not a valid `YYYY-MM-DD` date.
pub fn compute(entries: &[(String, i64)], start: &str) -> Option<(TimelogStatus, Vec<DayCell>)> {
    let start = parse_date(start)?;
    let today = today_local();
    Some((
        compute_status(entries, start, today),
        build_timeline(entries, start, today),
    ))
}

/// Compute the work-log status. `start` after `today` yields an empty green
/// result.
pub fn compute_status(
    entries: &[(String, i64)],
    start: NaiveDate,
    today: NaiveDate,
) -> TimelogStatus {
    if start > today {
        return TimelogStatus {
            logged_min: 0,
            planned_min: 0,
            today_deficit_min: 0,
            deficit_min: 0,
            surplus_min: 0,
            status: Status::Green,
        };
    }

    let cells = build_timeline(entries, start, today);
    let cur_week = monday_of(today);

    // Weekly shortfall keyed by that week's Monday, to judge "fully filled".
    let mut week_deficit: std::collections::HashMap<NaiveDate, i64> = std::collections::HashMap::new();
    for c in &cells {
        let d = NaiveDate::parse_from_str(&c.date, DATE_FMT).expect("cell date is valid");
        *week_deficit.entry(monday_of(d)).or_insert(0) += c.deficit_min;
    }

    let today_key = today.format(DATE_FMT).to_string();
    let mut logged = 0;
    let mut planned = 0;
    let mut today_deficit = 0;
    let mut deficit = 0;
    let mut surplus = 0;
    let mut prev_deficit = false;

    for c in &cells {
        let d = NaiveDate::parse_from_str(&c.date, DATE_FMT).expect("cell date is valid");
        let week = monday_of(d);
        let is_current_week = week == cur_week;
        // Drop fully-filled past weeks; the current week is always counted.
        if !is_current_week && week_deficit.get(&week) == Some(&0) {
            continue;
        }
        planned += c.plan_min;
        logged += c.logged_min;
        deficit += c.deficit_min;
        surplus += c.surplus_min;
        if c.date == today_key {
            today_deficit = c.deficit_min;
        } else if c.deficit_min > 0 {
            prev_deficit = true;
        }
    }

    let status = if prev_deficit {
        Status::Red
    } else if today_deficit > 0 {
        Status::Yellow
    } else if surplus > 0 {
        Status::Over
    } else {
        Status::Green
    };

    TimelogStatus {
        logged_min: logged,
        planned_min: planned,
        today_deficit_min: today_deficit,
        deficit_min: deficit,
        surplus_min: surplus,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, DATE_FMT).unwrap()
    }

    fn e(day: &str, minutes: i64) -> (String, i64) {
        (day.to_string(), minutes)
    }

    #[test]
    fn parse_and_fmt() {
        assert_eq!(parse_date("2026-07-08"), Some(d("2026-07-08")));
        assert_eq!(parse_date("nope"), None);
        assert_eq!(fmt(d("2026-07-08")), "2026-07-08");
    }

    #[test]
    fn window_requires_a_configured_start() {
        assert_eq!(window(None), None);
        assert_eq!(window(Some("")), None);
        assert_eq!(window(Some("garbage")), None);
        let (start, _) = window(Some("2026-01-15")).unwrap();
        assert_eq!(start, d("2026-01-15"));
    }

    #[test]
    fn minutes_from_hours_rounds() {
        assert_eq!(minutes_from_hours(1.5), 90);
        assert_eq!(minutes_from_hours(8.0), 480);
        assert_eq!(minutes_from_hours(0.0), 0);
    }

    #[test]
    fn start_after_today_is_empty_green() {
        let s = compute_status(&[], d("2026-07-10"), d("2026-07-08"));
        assert_eq!(s.status, Status::Green);
        assert_eq!(s.planned_min, 0);
    }

    #[test]
    fn single_full_weekday_is_green() {
        // 2026-07-08 is a Wednesday.
        let s = compute_status(&[e("2026-07-08", 480)], d("2026-07-08"), d("2026-07-08"));
        assert_eq!(s.status, Status::Green);
        assert_eq!(s.planned_min, 480);
        assert_eq!(s.logged_min, 480);
        assert_eq!(s.deficit_min, 0);
    }

    #[test]
    fn shortfall_only_today_is_yellow() {
        let s = compute_status(&[e("2026-07-08", 300)], d("2026-07-08"), d("2026-07-08"));
        assert_eq!(s.status, Status::Yellow);
        assert_eq!(s.today_deficit_min, 180);
        assert_eq!(s.deficit_min, 180);
    }

    #[test]
    fn shortfall_on_past_day_is_red() {
        // Tue full, Wed (today) full, but Mon short.
        let entries = [
            e("2026-07-06", 300), // Mon short
            e("2026-07-07", 480), // Tue full
            e("2026-07-08", 480), // Wed full (today)
        ];
        let s = compute_status(&entries, d("2026-07-06"), d("2026-07-08"));
        assert_eq!(s.status, Status::Red);
        assert_eq!(s.today_deficit_min, 0);
        assert_eq!(s.deficit_min, 180);
    }

    #[test]
    fn overlog_without_deficit_is_over() {
        let s = compute_status(&[e("2026-07-08", 600)], d("2026-07-08"), d("2026-07-08"));
        assert_eq!(s.status, Status::Over);
        assert_eq!(s.surplus_min, 120);
        assert_eq!(s.deficit_min, 0);
    }

    #[test]
    fn overlog_does_not_offset_other_day_deficit() {
        // Mon overlogged, Wed (today) short: surplus does not cancel deficit.
        let entries = [e("2026-07-06", 600), e("2026-07-07", 480), e("2026-07-08", 300)];
        let s = compute_status(&entries, d("2026-07-06"), d("2026-07-08"));
        assert_eq!(s.deficit_min, 180);
        assert_eq!(s.surplus_min, 120);
        assert_eq!(s.status, Status::Yellow); // deficit only today
    }

    #[test]
    fn weekend_has_no_plan() {
        // 2026-07-11 is Saturday.
        let s = compute_status(&[], d("2026-07-11"), d("2026-07-11"));
        assert_eq!(s.planned_min, 0);
        assert_eq!(s.status, Status::Green);
    }

    #[test]
    fn fully_filled_past_week_is_dropped_from_totals() {
        // Previous full week (Mon-Fri 480 each) + current week today short.
        // Prev week: 2026-06-29..07-03 (Mon-Fri). Current: 2026-07-06 Mon = today.
        let mut entries = Vec::new();
        for day in ["2026-06-29", "2026-06-30", "2026-07-01", "2026-07-02", "2026-07-03"] {
            entries.push(e(day, 480));
        }
        entries.push(e("2026-07-06", 300)); // current Mon, short
        let s = compute_status(&entries, d("2026-06-29"), d("2026-07-06"));
        // Prev full week dropped: planned counts only the current Monday.
        assert_eq!(s.planned_min, 480);
        assert_eq!(s.logged_min, 300);
        assert_eq!(s.status, Status::Yellow);
    }

    #[test]
    fn rank_candidates_least_logged_first() {
        let c = |server: &str, wp: i64, logged: i64| Candidate {
            server: server.into(),
            wp_id: wp,
            subject: format!("wp{wp}"),
            logged_min: logged,
        };
        let ranked = rank_candidates(vec![c("a", 3, 120), c("a", 1, 0), c("b", 2, 0)]);
        // Least logged first; ties broken by wp id.
        assert_eq!(
            ranked.iter().map(|x| x.wp_id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn compute_rejects_bad_start_and_empty_status_is_green() {
        assert!(compute(&[], "not-a-date").is_none());
        assert_eq!(empty_status().status, Status::Green);
        assert_eq!(empty_status().planned_min, 0);
        // A valid start computes a (status, timeline) pair.
        let (_, timeline) = compute(&[], &fmt(today_local())).unwrap();
        assert_eq!(timeline.len(), 1); // start == today
    }

    #[test]
    fn timeline_marks_weekend_and_deficit() {
        // Fri short, Sat weekend, Sun weekend, Mon full.
        let entries = [e("2026-07-10", 300), e("2026-07-13", 480)];
        let cells = build_timeline(&entries, d("2026-07-10"), d("2026-07-13"));
        assert_eq!(cells.len(), 4);
        assert!(cells[0].weekday); // Fri
        assert_eq!(cells[0].deficit_min, 180);
        assert!(!cells[1].weekday); // Sat
        assert_eq!(cells[1].plan_min, 0);
        assert!(!cells[2].weekday); // Sun
        assert!(cells[3].weekday); // Mon
        assert_eq!(cells[3].deficit_min, 0);
    }
}
