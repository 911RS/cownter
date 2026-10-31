use crate::model::Entry;
use crate::store::{DAY_MS, Result, Store};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

pub const RECAP_DAYS: i64 = 7;
pub const RECAP_MAX: usize = 6;
/// Local hour after which the weekly recap may show.
pub const RECAP_HOUR: u32 = 9;
const LAST_KEY: &str = "recap_week";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestDay {
    /// Start of the local day, in ms.
    pub day: i64,
    pub gain: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecapItem {
    pub entry: Entry,
    pub value: f64,
    /// Value now minus the value at the start of the week.
    pub change: f64,
    pub best_day: Option<BestDay>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recap {
    pub since: i64,
    pub until: i64,
    pub items: Vec<RecapItem>,
}

/// Splits `[since, now]` at local midnights: `(day start, from, to)` for each day touched.
pub fn day_spans(since: i64, now: i64, day_start: i64) -> Vec<(i64, i64, i64)> {
    let mut day = day_start;
    while day > since {
        day -= DAY_MS;
    }
    let mut out = vec![];
    while day < now {
        let (from, to) = (day.max(since), (day + DAY_MS).min(now));
        if from < to {
            out.push((day, from, to));
        }
        day += DAY_MS;
    }
    out
}

/// The day with the largest positive gain; ties go to the later day.
pub fn best_day(gains: &[(i64, f64)]) -> Option<BestDay> {
    gains
        .iter()
        .filter(|(_, g)| g.is_finite() && *g > 0.0)
        .fold(None::<(i64, f64)>, |best, &(d, g)| match best {
            Some((_, b)) if b > g => best,
            _ => Some((d, g)),
        })
        .map(|(day, gain)| BestDay { day, gain })
}

/// Largest absolute change first, at most `cap` items.
pub fn rank(mut items: Vec<RecapItem>, cap: usize) -> Vec<RecapItem> {
    items.sort_by(|a, b| b.change.abs().total_cmp(&a.change.abs()));
    items.truncate(cap);
    items
}

fn entry_recap(store: &Store, entry: Entry, since: i64, now: i64, day_start: i64) -> Result<Option<RecapItem>> {
    if entry.text_mode {
        return Ok(None);
    }
    let id = entry.id;
    let Some(value) = store.value_at_or_before(id, now)? else { return Ok(None) };
    let Some(base) = store.value_at_or_before(id, since)?.or(store.first_value_since(id, since)?) else { return Ok(None) };
    let mut gains = vec![];
    for (day, from, to) in day_spans(since, now, day_start) {
        let Some(close) = store.value_at_or_before(id, to)? else { continue };
        let open = store.value_at_or_before(id, from)?.unwrap_or(base);
        gains.push((day, close - open));
    }
    Ok(Some(RecapItem { change: value - base, value, best_day: best_day(&gains), entry }))
}

/// Change over the last seven days for every numeric entry, biggest movers first.
pub fn weekly_recap(store: &Store, now: i64, day_start: i64) -> Result<Recap> {
    let since = now - RECAP_DAYS * DAY_MS;
    let mut items = vec![];
    for e in store.entries()? {
        if let Some(it) = entry_recap(store, e, since, now, day_start)? {
            items.push(it);
        }
    }
    Ok(Recap { since, until: now, items: rank(items, RECAP_MAX) })
}

/// Date of the latest recap moment at or before `now`; `weekday` 0 is Monday.
pub fn recap_slot(now: NaiveDateTime, weekday: u32) -> NaiveDate {
    let today = now.date();
    let back = (today.weekday().num_days_from_monday() + 7 - weekday.min(6)) % 7;
    let mut d = today - Duration::days(back as i64);
    if back == 0 && now.time() < NaiveTime::from_hms_opt(RECAP_HOUR, 0, 0).unwrap_or_default() {
        d -= Duration::days(7);
    }
    d
}

/// Slot to remember and whether to show it now. The first check only remembers the slot.
pub fn recap_due(now: NaiveDateTime, weekday: u32, last: Option<&str>) -> Option<(NaiveDate, bool)> {
    let slot = recap_slot(now, weekday);
    match last.and_then(|l| NaiveDate::parse_from_str(l, "%Y-%m-%d").ok()) {
        None => Some((slot, false)),
        Some(l) if slot > l => Some((slot, true)),
        Some(_) => None,
    }
}

/// Checks and records the weekly slot in the store; true when the recap should show now.
pub fn take_due(store: &Store, now: NaiveDateTime, weekday: u32) -> Result<bool> {
    let last = store.setting(LAST_KEY)?;
    match recap_due(now, weekday, last.as_deref()) {
        Some((slot, show)) => {
            store.set_setting(LAST_KEY, &slot.format("%Y-%m-%d").to_string())?;
            Ok(show)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Reading, Status};
    use crate::store::tests::{entry, url_source};

    const T0: i64 = 1_800_000_000_000;

    fn put(s: &Store, id: i64, ts: i64, v: f64) {
        s.insert_reading(&Reading { entry_id: id, ts, value: Some(v), status: Status::Live, error: None, text: None }).unwrap();
    }

    fn at(date: &str, h: u32) -> NaiveDateTime {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap().and_hms_opt(h, 0, 0).unwrap()
    }

    fn item(slug: &str, change: f64) -> RecapItem {
        RecapItem { entry: entry(slug, url_source()), value: 0.0, change, best_day: None }
    }

    #[test]
    fn spans_cover_the_window() {
        let now = T0 + 9 * 3_600_000;
        let since = now - 7 * DAY_MS;
        let spans = day_spans(since, now, T0);
        assert_eq!(spans.len(), 8);
        assert_eq!(spans[0], (T0 - 7 * DAY_MS, since, T0 - 6 * DAY_MS));
        assert_eq!(spans[7], (T0, T0, now));
        assert!(spans.windows(2).all(|w| w[0].2 == w[1].1));
        assert_eq!(day_spans(T0, T0 + 10, T0), vec![(T0, T0, T0 + 10)]);
    }

    #[test]
    fn best_day_picks_largest_gain() {
        assert_eq!(best_day(&[]), None);
        assert_eq!(best_day(&[(1, -5.0), (2, 0.0)]), None);
        assert_eq!(best_day(&[(1, 3.0), (2, 9.0), (3, -20.0), (4, 4.0)]), Some(BestDay { day: 2, gain: 9.0 }));
        assert_eq!(best_day(&[(1, 5.0), (2, 5.0)]), Some(BestDay { day: 2, gain: 5.0 }));
        assert_eq!(best_day(&[(1, f64::NAN), (2, 1.0)]), Some(BestDay { day: 2, gain: 1.0 }));
    }

    #[test]
    fn rank_by_absolute_change() {
        let items = vec![item("a", 5.0), item("b", -50.0), item("c", 0.0), item("d", 20.0)];
        let slugs: Vec<String> = rank(items.clone(), 6).into_iter().map(|i| i.entry.slug).collect();
        assert_eq!(slugs, ["b", "d", "a", "c"]);
        assert_eq!(rank(items, 2).len(), 2);
    }

    #[test]
    fn recap_from_history() {
        let s = Store::open_memory().unwrap();
        let a = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        let b = s.insert_entry(&entry("b", url_source()), 0).unwrap();
        let mut t = entry("t", url_source());
        t.text_mode = true;
        s.insert_entry(&t, 0).unwrap();
        s.insert_entry(&entry("empty", url_source()), 0).unwrap();
        let now = T0 + 10 * 3_600_000;
        put(&s, a, now - 8 * DAY_MS, 100.0);
        put(&s, a, T0 - 5 * DAY_MS + 1000, 110.0);
        put(&s, a, T0 - 3 * DAY_MS + 1000, 150.0);
        put(&s, a, T0 - DAY_MS + 1000, 140.0);
        put(&s, a, T0 + 1000, 160.0);
        put(&s, b, T0 - 2 * DAY_MS + 1000, 10.0);
        put(&s, b, T0 - DAY_MS + 1000, 8.0);
        let r = weekly_recap(&s, now, T0).unwrap();
        assert_eq!((r.since, r.until), (now - 7 * DAY_MS, now));
        assert_eq!(r.items.len(), 2);
        let first = &r.items[0];
        assert_eq!(first.entry.slug, "a");
        assert_eq!(first.value, 160.0);
        assert_eq!(first.change, 60.0);
        assert_eq!(first.best_day, Some(BestDay { day: T0 - 3 * DAY_MS, gain: 40.0 }));
        let second = &r.items[1];
        assert_eq!(second.entry.slug, "b");
        assert_eq!(second.change, -2.0);
        assert_eq!(second.best_day, None);
    }

    #[test]
    fn slot_is_latest_weekday_after_nine() {
        assert_eq!(at("2026-11-02", 10).weekday().num_days_from_monday(), 0);
        assert_eq!(recap_slot(at("2026-11-02", 10), 0).to_string(), "2026-11-02");
        assert_eq!(recap_slot(at("2026-11-02", 8), 0).to_string(), "2026-10-26");
        assert_eq!(recap_slot(at("2026-11-05", 7), 0).to_string(), "2026-11-02");
        assert_eq!(recap_slot(at("2026-11-05", 12), 4).to_string(), "2026-10-30");
        assert_eq!(recap_slot(at("2026-11-08", 12), 6).to_string(), "2026-11-08");
    }

    #[test]
    fn due_once_per_slot() {
        let now = at("2026-11-02", 10);
        assert_eq!(recap_due(now, 0, None).map(|(d, show)| (d.to_string(), show)), Some(("2026-11-02".into(), false)));
        assert_eq!(recap_due(now, 0, Some("bad")).map(|x| x.1), Some(false));
        assert_eq!(recap_due(now, 0, Some("2026-10-26")).map(|(d, show)| (d.to_string(), show)), Some(("2026-11-02".into(), true)));
        assert_eq!(recap_due(now, 0, Some("2026-11-02")), None);
        assert_eq!(recap_due(at("2026-11-02", 8), 0, Some("2026-10-26")), None);
    }

    #[test]
    fn take_due_remembers_the_week() {
        let s = Store::open_memory().unwrap();
        assert!(!take_due(&s, at("2026-10-28", 12), 0).unwrap());
        assert!(!take_due(&s, at("2026-11-02", 8), 0).unwrap());
        assert!(take_due(&s, at("2026-11-02", 9), 0).unwrap());
        assert!(!take_due(&s, at("2026-11-02", 15), 0).unwrap());
        assert!(take_due(&s, at("2026-11-10", 15), 0).unwrap());
        assert_eq!(s.setting(LAST_KEY).unwrap().as_deref(), Some("2026-11-09"));
    }
}
