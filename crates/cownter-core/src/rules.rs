use crate::formula::{self, Eval, Input};
use crate::model::*;
use crate::store::{DAY_MS, Result, Store};
use std::collections::{HashMap, HashSet};

const SPARK_POINTS: usize = 24;
const FORECAST_DAYS: i64 = 30;
const FORECAST_POINTS: usize = 120;
const FORECAST_MIN_SPAN_MS: i64 = DAY_MS / 2;
const FORECAST_MAX_MS: f64 = 10.0 * 365.0 * DAY_MS as f64;
const MAX_STREAK_DAYS: u32 = 366;

/// When a linear trend through `points` takes `value` up to `goal`; None if reached, flat, falling or too far out.
pub fn goal_eta(points: &[(i64, f64)], value: f64, goal: f64, now: i64) -> Option<i64> {
    if value >= goal || points.len() < 2 {
        return None;
    }
    let t0 = points[0].0;
    if points[points.len() - 1].0 - t0 < FORECAST_MIN_SPAN_MS {
        return None;
    }
    let n = points.len() as f64;
    let mx = points.iter().map(|p| (p.0 - t0) as f64).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (t, v) in points {
        let dx = (t - t0) as f64 - mx;
        sxy += dx * (v - my);
        sxx += dx * dx;
    }
    let slope = sxy / sxx;
    if !slope.is_finite() || slope <= 0.0 {
        return None;
    }
    let ms = (goal - value) / slope;
    if ms > FORECAST_MAX_MS { None } else { Some(now + ms.ceil() as i64) }
}

/// Days in a row that closed higher than the day before. `close(0)` is the current value,
/// `close(k)` the value at the end of the day `k` days ago. A flat today does not break the run.
pub fn day_streak<E>(mut close: impl FnMut(u32) -> std::result::Result<Option<f64>, E>) -> std::result::Result<u32, E> {
    let Some(mut cur) = close(0)? else { return Ok(0) };
    let mut n = 0;
    for k in 1..=MAX_STREAK_DAYS {
        let Some(prev) = close(k)? else { break };
        if cur > prev {
            n += 1;
        } else if !(k == 1 && cur == prev) {
            break;
        }
        cur = prev;
    }
    Ok(n)
}

pub fn build_view(store: &Store, entry: Entry, now: i64, day_start: i64) -> Result<EntryView> {
    let id = entry.id;
    let latest = store.latest(id)?;
    let lv = store.latest_value(id)?;
    let value = lv.map(|(_, v)| v);
    let (status, error, updated_at) = match &latest {
        Some(r) => (r.status, r.error.clone(), Some(r.ts)),
        None => (Status::Polling, None, None),
    };
    let delta_today = match value {
        Some(v) => store
            .value_at_or_before(id, day_start)?
            .or(store.first_value_since(id, day_start)?)
            .map(|base| v - base),
        None => None,
    };
    let (text, prev_text) = if entry.text_mode { store.last_texts(id)? } else { (None, None) };
    let goal_progress = match (value, entry.goal_value) {
        (Some(v), Some(g)) if g > 0.0 => Some((v / g).clamp(0.0, 1.0)),
        _ => None,
    };
    let goal_eta = match (value, entry.goal_value) {
        (Some(v), Some(g)) => goal_eta(&store.spark(id, now - FORECAST_DAYS * DAY_MS, now, FORECAST_POINTS)?, v, g, now),
        _ => None,
    };
    let streak = if entry.text_mode {
        0
    } else {
        day_streak(|k| if k == 0 { Ok(value) } else { store.value_at_or_before(id, day_start - (k as i64 - 1) * DAY_MS - 1) })?
    };
    Ok(EntryView {
        value,
        prev: store.previous_value(id)?,
        status,
        error,
        updated_at,
        delta_today,
        spark: store.spark(id, now - DAY_MS, now, SPARK_POINTS)?,
        goal_progress,
        goal_eta,
        streak,
        text,
        prev_text,
        entry,
    })
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v:.2}") }
}

fn fmt_window(s: u32) -> String {
    let (n, unit) = match s {
        s if s % 86400 == 0 => (s / 86400, "day"),
        s if s % 3600 == 0 => (s / 3600, "hour"),
        s if s % 60 == 0 => (s / 60, "minute"),
        s => (s, "second"),
    };
    if n == 1 { format!("1 {unit}") } else { format!("{n} {unit}s") }
}

/// Alerts, goal, all-time high and change events for a fresh live value.
fn value_events(store: &Store, entry: &Entry, value: f64, prev: Option<f64>, ts: i64) -> Result<Vec<CoreEvent>> {
    let mut out = Vec::new();
    let id = entry.id;
    if let Some(p) = prev {
        if p != value {
            out.push(CoreEvent::ValueChanged { entry_id: id, value, prev: p });
        }
    }
    for (a, armed) in store.alerts_with_state(id)? {
        if !a.enabled {
            continue;
        }
        match a.kind {
            AlertKind::Above | AlertKind::Below => {
                let hit = if a.kind == AlertKind::Above { value > a.threshold } else { value < a.threshold };
                if hit && armed {
                    store.set_alert_state(a.id, false, Some(ts))?;
                    let word = if a.kind == AlertKind::Above { "above" } else { "below" };
                    out.push(CoreEvent::AlertFired {
                        entry_id: id,
                        alert_id: a.id,
                        message: format!("{} is {} {} ({})", entry.name, word, fmt_num(a.threshold), fmt_num(value)),
                        value,
                        diff: prev.map(|p| value - p),
                        notify: a.notify,
                        effect: a.effect.clone(),
                    });
                } else if !hit && !armed {
                    store.set_alert_state(a.id, true, None)?;
                }
            }
            AlertKind::ChangePct => {
                let window_ms = a.window_s as i64 * 1000;
                let cooled = a.last_fired_at.is_none_or(|t| ts - t >= window_ms);
                if let Some(old) = store.value_at_or_before(id, ts - window_ms)? {
                    if old != 0.0 && cooled {
                        let pct = (value - old) / old.abs() * 100.0;
                        if pct.abs() >= a.threshold {
                            store.set_alert_state(a.id, true, Some(ts))?;
                            let sign = if pct > 0.0 { "+" } else { "" };
                            out.push(CoreEvent::AlertFired {
                                entry_id: id,
                                alert_id: a.id,
                                message: format!("{} changed {sign}{pct:.1}% ({})", entry.name, fmt_num(value)),
                                value,
                                diff: Some(value - old),
                                notify: a.notify,
                                effect: a.effect.clone(),
                            });
                        }
                    }
                }
            }
            AlertKind::RisePct | AlertKind::DropPct => {
                let up = a.kind == AlertKind::RisePct;
                let since = ts - a.window_s as i64 * 1000;
                let mut points: Vec<f64> = store.history(id, since, ts)?.into_iter().map(|(_, v)| v).collect();
                points.extend(store.value_at_or_before(id, since)?);
                let reference = if up {
                    points.iter().copied().reduce(f64::min)
                } else {
                    points.iter().copied().reduce(f64::max)
                };
                let Some(r) = reference.filter(|r| *r != 0.0) else { continue };
                let pct = (value - r) / r.abs() * 100.0;
                let hit = if up { pct >= a.threshold } else { -pct >= a.threshold };
                if hit && armed {
                    store.set_alert_state(a.id, false, Some(ts))?;
                    let word = if up { "rose" } else { "dropped" };
                    out.push(CoreEvent::AlertFired {
                        entry_id: id,
                        alert_id: a.id,
                        message: format!(
                            "{} {word} {:.1}% within {} ({})",
                            entry.name,
                            pct.abs(),
                            fmt_window(a.window_s),
                            fmt_num(value)
                        ),
                        value,
                        diff: Some(value - r),
                        notify: a.notify,
                        effect: a.effect.clone(),
                    });
                } else if !hit && !armed {
                    store.set_alert_state(a.id, true, None)?;
                }
            }
            AlertKind::RiseBy | AlertKind::DropBy => {
                let Some(p) = prev else { continue };
                let diff = value - p;
                let up = a.kind == AlertKind::RiseBy;
                if (up && diff >= a.threshold) || (!up && -diff >= a.threshold) {
                    store.set_alert_state(a.id, true, Some(ts))?;
                    let sign = if diff > 0.0 { "+" } else { "" };
                    out.push(CoreEvent::AlertFired {
                        entry_id: id,
                        alert_id: a.id,
                        message: format!("{} changed by {sign}{} ({})", entry.name, fmt_num(diff), fmt_num(value)),
                        value,
                        diff: Some(diff),
                        notify: a.notify,
                        effect: a.effect.clone(),
                    });
                }
            }
        }
    }
    if let Some(goal) = entry.goal_value {
        if value >= goal && store.mark(id, "goal")?.is_none() {
            store.set_mark(id, "goal", ts)?;
            out.push(CoreEvent::GoalReached { entry_id: id, value, text: None });
        }
    }
    if let Some(max) = store.max_value_before(id, ts)? {
        let recent = store.mark(id, "ath")?.is_some_and(|t| ts - t < DAY_MS);
        if value > max && !recent {
            store.set_mark(id, "ath", ts)?;
            out.push(CoreEvent::AllTimeHigh { entry_id: id, value });
        }
    }
    Ok(out)
}

/// Change and goal events for a fresh text reading.
fn text_events(store: &Store, entry: &Entry, text: &str, ts: i64) -> Result<Vec<CoreEvent>> {
    let mut out = Vec::new();
    let id = entry.id;
    if let (_, Some(prev)) = store.last_texts(id)? {
        if prev != text {
            out.push(CoreEvent::TextChanged { entry_id: id, text: text.to_string(), prev });
        }
    }
    if let Some(goal) = &entry.goal_text {
        let hit = crate::parse::same_text(text, goal);
        let marked = store.mark(id, "goal")?.is_some();
        if hit && !marked {
            store.set_mark(id, "goal", ts)?;
            out.push(CoreEvent::GoalReached { entry_id: id, value: 0.0, text: Some(text.to_string()) });
        } else if !hit && marked {
            store.clear_mark(id, "goal")?;
        }
    }
    Ok(out)
}

fn input_for(store: &Store, slug: &str) -> Result<Option<Input>> {
    let Some(e) = store.entry_by_slug(slug)? else { return Ok(None) };
    if e.text_mode {
        return Ok(Some(Input::Error));
    }
    let latest = store.latest(e.id)?;
    Ok(Some(match latest {
        Some(Reading { status: Status::Live, value: Some(v), .. }) => Input::Value(v),
        Some(Reading { status: Status::Error | Status::NoLink, .. }) => Input::Error,
        _ => Input::Stale,
    }))
}

/// Formula entries that depend on `slug`, directly or transitively, in evaluation order.
fn dependents(store: &Store, slug: &str) -> Result<Vec<Entry>> {
    let formulas: Vec<(Entry, Vec<String>)> = store
        .entries()?
        .into_iter()
        .filter_map(|e| match &e.source {
            SourceCfg::Formula { expr } => {
                let r = formula::parse(expr).map(|x| formula::refs(&x)).unwrap_or_default();
                Some((e, r))
            }
            _ => None,
        })
        .collect();
    let mut affected: HashSet<String> = HashSet::new();
    let mut frontier = vec![slug.to_string()];
    while let Some(s) = frontier.pop() {
        for (e, r) in &formulas {
            if r.contains(&s) && affected.insert(e.slug.clone()) {
                frontier.push(e.slug.clone());
            }
        }
    }
    let by_slug: HashMap<&str, &(Entry, Vec<String>)> = formulas.iter().map(|f| (f.0.slug.as_str(), f)).collect();
    let mut order = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    fn visit(
        s: &str,
        by: &HashMap<&str, &(Entry, Vec<String>)>,
        affected: &HashSet<String>,
        done: &mut HashSet<String>,
        order: &mut Vec<Entry>,
        depth: usize,
    ) {
        if depth > 64 || done.contains(s) || !affected.contains(s) {
            return;
        }
        done.insert(s.to_string());
        if let Some((e, refs)) = by.get(s) {
            for r in refs {
                visit(r, by, affected, done, order, depth + 1);
            }
            order.push(e.clone());
        }
    }
    let mut names: Vec<&String> = affected.iter().collect();
    names.sort();
    for s in names {
        visit(s, &by_slug, &affected, &mut done, &mut order, 0);
    }
    Ok(order)
}

/// Evaluates one formula entry, stores the result and returns its events.
pub fn recompute_formula(store: &Store, entry: &Entry, now: i64, day_start: i64) -> Result<Vec<CoreEvent>> {
    let SourceCfg::Formula { expr } = &entry.source else { return Ok(vec![]) };
    let prev = store.latest_value(entry.id)?.map(|(_, v)| v);
    let (value, status, error) = match formula::parse(expr) {
        Err(e) => (None, Status::Error, Some(e)),
        Ok(ast) => {
            let mut inputs = HashMap::new();
            for r in formula::refs(&ast) {
                inputs.insert(r.clone(), input_for(store, &r)?);
            }
            match formula::eval(&ast, &|s| inputs.get(s).copied().flatten()) {
                Eval::Value(v) => (Some(v), Status::Live, None),
                Eval::Stale => (None, Status::Stale, Some("an input is not up to date".into())),
                Eval::Error(m) => (None, Status::Error, Some(m)),
            }
        }
    };
    store.insert_reading(&Reading { entry_id: entry.id, ts: now, value, status, error, text: None })?;
    let mut out = match value {
        Some(v) => value_events(store, entry, v, prev, now)?,
        None => vec![],
    };
    out.insert(0, CoreEvent::EntryUpdated { view: build_view(store, entry.clone(), now, day_start)? });
    Ok(out)
}

/// Call after storing a reading for `entry_id`.
pub fn after_reading(store: &Store, entry_id: i64, now: i64, day_start: i64) -> Result<Vec<CoreEvent>> {
    let Some(entry) = store.entry(entry_id)? else { return Ok(vec![]) };
    let mut out = Vec::new();
    if let Some(r) = store.latest(entry_id)? {
        if let (Status::Live, Some(v)) = (r.status, r.value) {
            out.extend(value_events(store, &entry, v, store.previous_value(entry_id)?, r.ts)?);
        }
        if let (Status::Live, Some(t)) = (r.status, &r.text) {
            out.extend(text_events(store, &entry, t, r.ts)?);
        }
    }
    out.insert(0, CoreEvent::EntryUpdated { view: build_view(store, entry.clone(), now, day_start)? });
    for f in dependents(store, &entry.slug)? {
        out.extend(recompute_formula(store, &f, now, day_start)?);
    }
    Ok(out)
}

/// Rejects formulas that do not parse, point at unknown entries or create a cycle.
pub fn validate_formula(store: &Store, slug: &str, expr: &str) -> std::result::Result<(), String> {
    let ast = formula::parse(expr)?;
    let refs = formula::refs(&ast);
    let entries = store.entries().map_err(|e| e.to_string())?;
    let known: HashSet<&str> = entries.iter().map(|e| e.slug.as_str()).collect();
    for r in &refs {
        if r != slug && !known.contains(r.as_str()) {
            return Err(format!("unknown entry `{r}`"));
        }
        if entries.iter().any(|e| &e.slug == r && e.text_mode) {
            return Err(format!("`{r}` shows text, so it cannot be used in a formula"));
        }
    }
    let mut graph: HashMap<String, Vec<String>> = entries
        .iter()
        .filter(|e| e.slug != slug)
        .filter_map(|e| match &e.source {
            SourceCfg::Formula { expr } => {
                Some((e.slug.clone(), formula::parse(expr).map(|x| formula::refs(&x)).unwrap_or_default()))
            }
            _ => None,
        })
        .collect();
    graph.insert(slug.to_string(), refs);
    match formula::find_cycle(&graph) {
        Some(c) => Err(format!("formula loops back on itself: {}", c.join(" → "))),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::{entry, url_source};

    const T0: i64 = 1_800_000_000_000;

    fn live(s: &Store, id: i64, ts: i64, v: f64) -> Vec<CoreEvent> {
        s.insert_reading(&Reading { entry_id: id, ts, value: Some(v), status: Status::Live, error: None, text: None }).unwrap();
        after_reading(s, id, ts, T0).unwrap()
    }

    fn count<F: Fn(&CoreEvent) -> bool>(evs: &[CoreEvent], f: F) -> usize {
        evs.iter().filter(|e| f(e)).count()
    }

    #[test]
    fn delta_today() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        live(&s, id, T0 - 3_600_000, 100.0);
        let evs = live(&s, id, T0 + 1000, 124.0);
        let CoreEvent::EntryUpdated { view } = &evs[0] else { panic!() };
        assert_eq!(view.delta_today, Some(24.0));
        assert_eq!(view.prev, Some(100.0));
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::ValueChanged { .. })), 1);
    }

    #[test]
    fn delta_today_new_entry_uses_first_reading() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        live(&s, id, T0 + 10, 50.0);
        let evs = live(&s, id, T0 + 20, 53.0);
        let CoreEvent::EntryUpdated { view } = &evs[0] else { panic!() };
        assert_eq!(view.delta_today, Some(3.0));
    }

    #[test]
    fn eta_from_linear_trend() {
        let pts: Vec<(i64, f64)> = (0..10).map(|d| (T0 + d * DAY_MS, 100.0 + 10.0 * d as f64)).collect();
        let now = T0 + 9 * DAY_MS;
        assert_eq!(goal_eta(&pts, 190.0, 240.0, now), Some(now + 5 * DAY_MS));
    }

    #[test]
    fn eta_none_cases() {
        let now = T0 + 9 * DAY_MS;
        let up: Vec<(i64, f64)> = (0..10).map(|d| (T0 + d * DAY_MS, 100.0 + d as f64)).collect();
        let flat: Vec<(i64, f64)> = (0..10).map(|d| (T0 + d * DAY_MS, 100.0)).collect();
        let down: Vec<(i64, f64)> = (0..10).map(|d| (T0 + d * DAY_MS, 100.0 - d as f64)).collect();
        assert_eq!(goal_eta(&[], 100.0, 200.0, now), None);
        assert_eq!(goal_eta(&up[..1], 100.0, 200.0, now), None);
        assert_eq!(goal_eta(&[(T0, 1.0), (T0 + 60_000, 2.0)], 2.0, 10.0, now), None);
        assert_eq!(goal_eta(&flat, 100.0, 200.0, now), None);
        assert_eq!(goal_eta(&down, 91.0, 200.0, now), None);
        assert_eq!(goal_eta(&up, 109.0, 100.0, now), None);
        assert_eq!(goal_eta(&up, 109.0, 109.0, now), None);
        assert_eq!(goal_eta(&up, 109.0, 1e9, now), None);
    }

    #[test]
    fn eta_in_view() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("a", url_source());
        e.goal_value = Some(200.0);
        let id = s.insert_entry(&e, 0).unwrap();
        for d in 0..5 {
            s.insert_reading(&Reading { entry_id: id, ts: T0 - (4 - d) * DAY_MS, value: Some(100.0 + 10.0 * d as f64), status: Status::Live, error: None, text: None }).unwrap();
        }
        let v = build_view(&s, s.entry(id).unwrap().unwrap(), T0, T0).unwrap();
        let eta = v.goal_eta.expect("eta");
        assert!((eta - (T0 + 6 * DAY_MS)).abs() < DAY_MS / 2, "{eta}");
        let mut none = s.entry(id).unwrap().unwrap();
        none.goal_value = None;
        assert_eq!(build_view(&s, none, T0, T0).unwrap().goal_eta, None);
    }

    fn streak_of(closes: &[f64]) -> u32 {
        day_streak::<()>(|k| Ok(closes.get(k as usize).copied())).unwrap()
    }

    #[test]
    fn streak_math() {
        assert_eq!(day_streak::<()>(|_| Ok(None)).unwrap(), 0);
        assert_eq!(streak_of(&[5.0]), 0);
        assert_eq!(streak_of(&[5.0, 5.0, 5.0]), 0);
        assert_eq!(streak_of(&[1.0, 2.0, 3.0, 4.0]), 0);
        assert_eq!(streak_of(&[4.0, 3.0, 2.0, 1.0]), 3);
        assert_eq!(streak_of(&[4.0, 3.0, 3.0, 1.0]), 1);
        assert_eq!(streak_of(&[3.0, 3.0, 2.0, 1.0]), 2);
        assert_eq!(streak_of(&[2.0, 3.0, 2.0, 1.0]), 0);
    }

    #[test]
    fn streak_in_view() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        for (d, v) in [(4, 10.0), (3, 8.0), (2, 9.0), (1, 12.0)] {
            s.insert_reading(&Reading { entry_id: id, ts: T0 - d * DAY_MS + 1000, value: Some(v), status: Status::Live, error: None, text: None }).unwrap();
        }
        let e = s.entry(id).unwrap().unwrap();
        assert_eq!(build_view(&s, e.clone(), T0 + 10, T0).unwrap().streak, 2);
        live(&s, id, T0 + 1000, 15.0);
        assert_eq!(build_view(&s, e.clone(), T0 + 2000, T0).unwrap().streak, 3);
        live(&s, id, T0 + 3000, 11.0);
        assert_eq!(build_view(&s, e, T0 + 4000, T0).unwrap().streak, 0);
    }

    #[test]
    fn above_fires_once_on_crossing() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Above, threshold: 50.0, window_s: 60, enabled: true, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
        let mut n = 0;
        for (i, v) in [40.0, 55.0, 60.0].iter().enumerate() {
            n += count(&live(&s, id, T0 + i as i64 * 1000, *v), |e| matches!(e, CoreEvent::AlertFired { .. }));
        }
        assert_eq!(n, 1);
        live(&s, id, T0 + 5000, 40.0);
        assert_eq!(count(&live(&s, id, T0 + 6000, 70.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 1);
    }

    fn fired(evs: &[CoreEvent]) -> usize {
        count(evs, |e| matches!(e, CoreEvent::AlertFired { .. }))
    }

    fn pct_alert(s: &Store, id: i64, kind: AlertKind, threshold: f64, window_s: u32) {
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind, threshold, window_s, enabled: true, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
    }

    const MIN: i64 = 60_000;

    #[test]
    fn below_fires_once_on_crossing() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Below, threshold: 10.0, window_s: 60, enabled: true, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
        let n: usize = [12.0, 9.0, 8.0, 7.0].iter().enumerate().map(|(i, v)| fired(&live(&s, id, T0 + i as i64 * 1000, *v))).sum();
        assert_eq!(n, 1);
        live(&s, id, T0 + 5000, 11.0);
        assert_eq!(fired(&live(&s, id, T0 + 6000, 9.5)), 1);
    }

    #[test]
    fn drop_pct_fires_once_per_crossing() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        pct_alert(&s, id, AlertKind::DropPct, 5.0, 3600);
        assert_eq!(fired(&live(&s, id, T0, 100.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 10 * MIN, 98.0)), 0);
        let evs = live(&s, id, T0 + 20 * MIN, 94.0);
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::AlertFired { message, diff: Some(d), .. } if message == "a dropped 6.0% within 1 hour (94)" && *d == -6.0)), 1);
        assert_eq!(fired(&live(&s, id, T0 + 30 * MIN, 90.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 40 * MIN, 99.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 50 * MIN, 93.0)), 1);
        assert_eq!(fired(&live(&s, id, T0 + 55 * MIN, 92.0)), 0);
    }

    #[test]
    fn drop_pct_rearms_when_the_peak_leaves_the_window() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        pct_alert(&s, id, AlertKind::DropPct, 5.0, 900);
        live(&s, id, T0, 100.0);
        assert_eq!(fired(&live(&s, id, T0 + 5 * MIN, 90.0)), 1);
        assert_eq!(fired(&live(&s, id, T0 + 30 * MIN, 90.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 35 * MIN, 85.0)), 1);
    }

    #[test]
    fn rise_pct_fires_once_per_crossing() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        pct_alert(&s, id, AlertKind::RisePct, 10.0, 3600);
        live(&s, id, T0, 200.0);
        assert_eq!(fired(&live(&s, id, T0 + 10 * MIN, 210.0)), 0);
        let evs = live(&s, id, T0 + 20 * MIN, 222.0);
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::AlertFired { message, .. } if message == "a rose 11.0% within 1 hour (222)")), 1);
        assert_eq!(fired(&live(&s, id, T0 + 30 * MIN, 240.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 40 * MIN, 205.0)), 0);
        assert_eq!(fired(&live(&s, id, T0 + 50 * MIN, 230.0)), 1);
    }

    #[test]
    fn pct_alerts_ignore_the_other_direction() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        pct_alert(&s, id, AlertKind::RisePct, 5.0, 3600);
        live(&s, id, T0, 100.0);
        assert_eq!(fired(&live(&s, id, T0 + MIN, 50.0)), 0);
        let s2 = Store::open_memory().unwrap();
        let id2 = s2.insert_entry(&entry("b", url_source()), 0).unwrap();
        pct_alert(&s2, id2, AlertKind::DropPct, 5.0, 3600);
        live(&s2, id2, T0, 100.0);
        assert_eq!(fired(&live(&s2, id2, T0 + MIN, 150.0)), 0);
    }

    #[test]
    fn rejects_bad_pct_alerts() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        let a = Alert { id: 0, entry_id: id, kind: AlertKind::DropPct, threshold: 0.0, window_s: 3600, enabled: true, last_fired_at: None, notify: true, effect: String::new() };
        assert!(s.upsert_alert(&a).is_err());
        assert!(s.upsert_alert(&Alert { threshold: 5.0, window_s: 0, ..a.clone() }).is_err());
        assert!(s.upsert_alert(&Alert { kind: AlertKind::RisePct, threshold: 5.0, ..a }).is_ok());
    }

    #[test]
    fn window_labels() {
        assert_eq!(fmt_window(900), "15 minutes");
        assert_eq!(fmt_window(3600), "1 hour");
        assert_eq!(fmt_window(21600), "6 hours");
        assert_eq!(fmt_window(604800), "7 days");
    }

    #[test]
    fn disabled_alert_never_fires() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Below, threshold: 5.0, window_s: 60, enabled: false, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
        assert_eq!(count(&live(&s, id, T0, 1.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 0);
    }

    #[test]
    fn change_pct_window() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::ChangePct, threshold: 10.0, window_s: 3600, enabled: true, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
        live(&s, id, T0, 100.0);
        let evs = live(&s, id, T0 + 3_600_000, 89.0);
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::AlertFired { message, .. } if message.contains("-11.0%"))), 1);
        assert_eq!(count(&live(&s, id, T0 + 3_700_000, 80.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 0);
    }

    #[test]
    fn goal_reached_once() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("a", url_source());
        e.goal_value = Some(50.0);
        let id = s.insert_entry(&e, 0).unwrap();
        let mut n = 0;
        for (i, v) in [49.0, 50.0, 51.0, 40.0, 60.0].iter().enumerate() {
            n += count(&live(&s, id, T0 + i as i64, *v), |e| matches!(e, CoreEvent::GoalReached { .. }));
        }
        assert_eq!(n, 1);
    }

    #[test]
    fn all_time_high_at_most_daily() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        assert_eq!(count(&live(&s, id, T0, 10.0), |e| matches!(e, CoreEvent::AllTimeHigh { .. })), 0);
        assert_eq!(count(&live(&s, id, T0 + 1, 11.0), |e| matches!(e, CoreEvent::AllTimeHigh { .. })), 1);
        assert_eq!(count(&live(&s, id, T0 + 2, 12.0), |e| matches!(e, CoreEvent::AllTimeHigh { .. })), 0);
        assert_eq!(count(&live(&s, id, T0 + DAY_MS + 2, 13.0), |e| matches!(e, CoreEvent::AllTimeHigh { .. })), 1);
    }

    #[test]
    fn formula_recomputed() {
        let s = Store::open_memory().unwrap();
        let rev = s.insert_entry(&entry("revenue", url_source()), 0).unwrap();
        let ord = s.insert_entry(&entry("orders", url_source()), 0).unwrap();
        let avg = s.insert_entry(&entry("avg", SourceCfg::Formula { expr: "revenue / orders".into() }), 0).unwrap();
        live(&s, rev, T0, 2940.0);
        let evs = live(&s, ord, T0 + 1, 37.0);
        let v = evs.iter().find_map(|e| match e {
            CoreEvent::EntryUpdated { view } if view.entry.id == avg => Some(view.clone()),
            _ => None,
        });
        let v = v.expect("formula updated");
        assert_eq!(v.status, Status::Live);
        assert!((v.value.unwrap() - 79.459).abs() < 0.01);

        s.insert_reading(&Reading { entry_id: ord, ts: T0 + 2, value: None, status: Status::Error, error: Some("x".into()), text: None }).unwrap();
        after_reading(&s, ord, T0 + 2, T0).unwrap();
        assert_eq!(s.latest(avg).unwrap().unwrap().status, Status::Error);

        s.insert_reading(&Reading { entry_id: ord, ts: T0 + 3, value: None, status: Status::Stale, error: None, text: None }).unwrap();
        after_reading(&s, ord, T0 + 3, T0).unwrap();
        let r = s.latest(avg).unwrap().unwrap();
        assert_eq!((r.status, r.value), (Status::Stale, None));
    }

    #[test]
    fn chained_formulas_in_order() {
        let s = Store::open_memory().unwrap();
        let a = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.insert_entry(&entry("b", SourceCfg::Formula { expr: "a * 2".into() }), 0).unwrap();
        let c = s.insert_entry(&entry("c", SourceCfg::Formula { expr: "b + a".into() }), 0).unwrap();
        live(&s, a, T0, 5.0);
        assert_eq!(s.latest(c).unwrap().unwrap().value, Some(15.0));
    }

    #[test]
    fn formula_validation() {
        let s = Store::open_memory().unwrap();
        s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.insert_entry(&entry("b", SourceCfg::Formula { expr: "c + 1".into() }), 0).unwrap();
        assert!(validate_formula(&s, "c", "a * 2").is_ok());
        assert_eq!(validate_formula(&s, "c", "zz").unwrap_err(), "unknown entry `zz`");
        assert!(validate_formula(&s, "c", "b + 1").unwrap_err().starts_with("formula loops back"));
        assert!(validate_formula(&s, "c", "c + 1").is_err());
        assert!(validate_formula(&s, "c", "a +").is_err());
    }

    #[test]
    fn step_alerts_fire_on_each_big_enough_change() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("steps", url_source()), T0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::RiseBy, threshold: 1.0, window_s: 60, enabled: true, last_fired_at: None, notify: false, effect: "pop".into() }).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::DropBy, threshold: 2.0, window_s: 60, enabled: true, last_fired_at: None, notify: true, effect: String::new() }).unwrap();
        live(&s, id, T0, 10.0);
        let evs = live(&s, id, T0 + 1000, 11.0);
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::AlertFired { diff: Some(d), notify: false, effect, .. } if *d == 1.0 && effect == "pop")), 1);
        assert_eq!(count(&live(&s, id, T0 + 2000, 12.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 1);
        assert_eq!(count(&live(&s, id, T0 + 3000, 11.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 0);
        assert_eq!(count(&live(&s, id, T0 + 4000, 8.0), |e| matches!(e, CoreEvent::AlertFired { notify: true, .. })), 1);
    }

    #[test]
    fn rejects_bad_step_alerts() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("bad", url_source()), T0).unwrap();
        let a = Alert { id: 0, entry_id: id, kind: AlertKind::RiseBy, threshold: 0.0, window_s: 60, enabled: true, last_fired_at: None, notify: true, effect: String::new() };
        assert!(s.upsert_alert(&a).is_err());
        assert!(s.upsert_alert(&Alert { threshold: 1.0, effect: "boom".into(), ..a }).is_err());
    }

    fn text_reading(s: &Store, id: i64, ts: i64, t: &str) -> Vec<CoreEvent> {
        s.insert_reading(&Reading { entry_id: id, ts, value: None, status: Status::Live, error: None, text: Some(t.into()) }).unwrap();
        after_reading(s, id, ts, T0).unwrap()
    }

    #[test]
    fn text_entries_report_changes_and_goal() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("order", url_source());
        e.text_mode = true;
        e.goal_text = Some("Delivered".into());
        let id = s.insert_entry(&e, 0).unwrap();
        let evs = text_reading(&s, id, T0, "Packed");
        let CoreEvent::EntryUpdated { view } = &evs[0] else { panic!() };
        assert_eq!(view.text.as_deref(), Some("Packed"));
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::TextChanged { .. })), 0);
        assert_eq!(count(&text_reading(&s, id, T0 + 1, "Packed"), |e| matches!(e, CoreEvent::TextChanged { .. })), 0);
        let evs = text_reading(&s, id, T0 + 2, "Shipped");
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::TextChanged { prev, text, .. } if prev == "Packed" && text == "Shipped")), 1);
        let evs = text_reading(&s, id, T0 + 3, " delivered ");
        assert_eq!(count(&evs, |e| matches!(e, CoreEvent::GoalReached { text: Some(_), .. })), 1);
        assert_eq!(count(&text_reading(&s, id, T0 + 4, "delivered"), |e| matches!(e, CoreEvent::GoalReached { .. })), 0);
        text_reading(&s, id, T0 + 5, "Returned");
        assert_eq!(count(&text_reading(&s, id, T0 + 6, "Delivered"), |e| matches!(e, CoreEvent::GoalReached { .. })), 1);
    }

    #[test]
    fn formulas_cannot_use_text_entries() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("status", url_source());
        e.text_mode = true;
        s.insert_entry(&e, 0).unwrap();
        assert!(validate_formula(&s, "f", "status + 1").unwrap_err().contains("text"));
    }

    #[test]
    fn text_mode_needs_a_url_source() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("calc", SourceCfg::Formula { expr: "1".into() });
        e.text_mode = true;
        assert!(s.insert_entry(&e, 0).is_err());
    }
}
