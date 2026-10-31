use crate::formula::{self, Eval, Input};
use crate::model::*;
use crate::store::{DAY_MS, Result, Store};
use std::collections::{HashMap, HashSet};

const SPARK_POINTS: usize = 24;

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
    let goal_progress = match (value, entry.goal_value) {
        (Some(v), Some(g)) if g > 0.0 => Some((v / g).clamp(0.0, 1.0)),
        _ => None,
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
        entry,
    })
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v:.2}") }
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
                            });
                        }
                    }
                }
            }
        }
    }
    if let Some(goal) = entry.goal_value {
        if value >= goal && store.mark(id, "goal")?.is_none() {
            store.set_mark(id, "goal", ts)?;
            out.push(CoreEvent::GoalReached { entry_id: id, value });
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

fn input_for(store: &Store, slug: &str) -> Result<Option<Input>> {
    let Some(e) = store.entry_by_slug(slug)? else { return Ok(None) };
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
    store.insert_reading(&Reading { entry_id: entry.id, ts: now, value, status, error })?;
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
        s.insert_reading(&Reading { entry_id: id, ts, value: Some(v), status: Status::Live, error: None }).unwrap();
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
    fn above_fires_once_on_crossing() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Above, threshold: 50.0, window_s: 60, enabled: true, last_fired_at: None }).unwrap();
        let mut n = 0;
        for (i, v) in [40.0, 55.0, 60.0].iter().enumerate() {
            n += count(&live(&s, id, T0 + i as i64 * 1000, *v), |e| matches!(e, CoreEvent::AlertFired { .. }));
        }
        assert_eq!(n, 1);
        live(&s, id, T0 + 5000, 40.0);
        assert_eq!(count(&live(&s, id, T0 + 6000, 70.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 1);
    }

    #[test]
    fn disabled_alert_never_fires() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::Below, threshold: 5.0, window_s: 60, enabled: false, last_fired_at: None }).unwrap();
        assert_eq!(count(&live(&s, id, T0, 1.0), |e| matches!(e, CoreEvent::AlertFired { .. })), 0);
    }

    #[test]
    fn change_pct_window() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.upsert_alert(&Alert { id: 0, entry_id: id, kind: AlertKind::ChangePct, threshold: 10.0, window_s: 3600, enabled: true, last_fired_at: None }).unwrap();
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

        s.insert_reading(&Reading { entry_id: ord, ts: T0 + 2, value: None, status: Status::Error, error: Some("x".into()) }).unwrap();
        after_reading(&s, ord, T0 + 2, T0).unwrap();
        assert_eq!(s.latest(avg).unwrap().unwrap().status, Status::Error);

        s.insert_reading(&Reading { entry_id: ord, ts: T0 + 3, value: None, status: Status::Stale, error: None }).unwrap();
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
}
