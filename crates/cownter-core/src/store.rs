use crate::model::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;

pub const HOUR_MS: i64 = 3_600_000;
pub const DAY_MS: i64 = 24 * HOUR_MS;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS groups(
  id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS entries(
  id INTEGER PRIMARY KEY,
  slug TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  icon_json TEXT NOT NULL,
  group_id INTEGER REFERENCES groups(id) ON DELETE SET NULL,
  source_cfg_json TEXT NOT NULL,
  interval_s INTEGER NOT NULL CHECK(interval_s >= 10),
  format_json TEXT NOT NULL,
  goal_value REAL,
  celebrate INTEGER NOT NULL DEFAULT 1,
  sounds_json TEXT NOT NULL DEFAULT '{}',
  sort INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  text_mode INTEGER NOT NULL DEFAULT 0,
  goal_text TEXT);
CREATE TABLE IF NOT EXISTS alerts(
  id INTEGER PRIMARY KEY,
  entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  threshold REAL NOT NULL,
  window_s INTEGER NOT NULL DEFAULT 3600,
  enabled INTEGER NOT NULL DEFAULT 1,
  last_fired_at INTEGER,
  armed INTEGER NOT NULL DEFAULT 1,
  notify INTEGER NOT NULL DEFAULT 1,
  effect TEXT NOT NULL DEFAULT '');
CREATE TABLE IF NOT EXISTS readings(
  entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  ts INTEGER NOT NULL,
  value REAL,
  status TEXT NOT NULL,
  error TEXT,
  text TEXT);
CREATE INDEX IF NOT EXISTS readings_entry_ts ON readings(entry_id, ts);
CREATE TABLE IF NOT EXISTS marks(
  entry_id INTEGER NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  ts INTEGER NOT NULL,
  PRIMARY KEY(entry_id, kind));
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
"#;

fn to_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("serializable")
}

fn from_json<T: serde::de::DeserializeOwned>(s: &str, col: usize) -> rusqlite::Result<T> {
    serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(col, rusqlite::types::Type::Text, Box::new(e)))
}

fn clean_goal_text(g: &Option<String>) -> Option<String> {
    g.as_ref().map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

const ENTRY_COLS: &str =
    "id, slug, name, icon_json, group_id, source_cfg_json, interval_s, format_json, goal_value, celebrate, sort, sounds_json, text_mode, goal_text";

fn row_entry(r: &rusqlite::Row) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?,
        slug: r.get(1)?,
        name: r.get(2)?,
        icon: from_json(&r.get::<_, String>(3)?, 3)?,
        group_id: r.get(4)?,
        source: from_json(&r.get::<_, String>(5)?, 5)?,
        interval_s: r.get(6)?,
        format: from_json(&r.get::<_, String>(7)?, 7)?,
        goal_value: r.get(8)?,
        celebrate: r.get::<_, i64>(9)? != 0,
        sort: r.get(10)?,
        sounds: from_json(&r.get::<_, String>(11)?, 11)?,
        text_mode: r.get::<_, i64>(12)? != 0,
        goal_text: r.get(13)?,
    })
}

fn row_reading(r: &rusqlite::Row) -> rusqlite::Result<Reading> {
    Ok(Reading {
        entry_id: r.get(0)?,
        ts: r.get(1)?,
        value: r.get(2)?,
        status: Status::parse(&r.get::<_, String>(3)?),
        error: r.get(4)?,
        text: r.get(5)?,
    })
}

fn row_alert(r: &rusqlite::Row) -> rusqlite::Result<(Alert, bool)> {
    let kind: String = r.get(2)?;
    Ok((
        Alert {
            id: r.get(0)?,
            entry_id: r.get(1)?,
            kind: AlertKind::parse(&kind).unwrap_or(AlertKind::Above),
            threshold: r.get(3)?,
            window_s: r.get(4)?,
            enabled: r.get::<_, i64>(5)? != 0,
            last_fired_at: r.get(6)?,
            notify: r.get::<_, i64>(8)? != 0,
            effect: r.get(9)?,
        },
        r.get::<_, i64>(7)? != 0,
    ))
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| StoreError::Invalid(e.to_string()))?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA)?;
        Self::migrate(&conn)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    fn has_column(conn: &Connection, table: &str, col: &str) -> Result<bool> {
        let mut st = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let names = st.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(names.iter().any(|n| n == col))
    }

    fn migrate(conn: &Connection) -> Result<()> {
        for (table, col, ddl) in [
            ("entries", "text_mode", "ALTER TABLE entries ADD COLUMN text_mode INTEGER NOT NULL DEFAULT 0"),
            ("entries", "goal_text", "ALTER TABLE entries ADD COLUMN goal_text TEXT"),
            ("readings", "text", "ALTER TABLE readings ADD COLUMN text TEXT"),
        ] {
            if !Self::has_column(conn, table, col)? {
                conn.execute_batch(ddl)?;
            }
        }
        if !Self::has_column(conn, "alerts", "notify")? {
            conn.execute_batch("ALTER TABLE alerts ADD COLUMN notify INTEGER NOT NULL DEFAULT 1")?;
        }
        if !Self::has_column(conn, "alerts", "effect")? {
            conn.execute_batch("ALTER TABLE alerts ADD COLUMN effect TEXT NOT NULL DEFAULT ''")?;
        }
        Ok(())
    }

    fn c(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn validate(e: &Entry) -> Result<()> {
        if !valid_slug(&e.slug) {
            return Err(StoreError::Invalid(
                "slug must start with a letter and use only a-z, 0-9 and _ (max 40)".into(),
            ));
        }
        if e.name.trim().is_empty() {
            return Err(StoreError::Invalid("name is required".into()));
        }
        if e.interval_s < 10 {
            return Err(StoreError::Invalid("poll interval must be at least 10 seconds".into()));
        }
        if e.text_mode && !e.source.allows_text() {
            return Err(StoreError::Invalid("text values only work with the Any URL and Push sources".into()));
        }
        if e.goal_text.as_ref().is_some_and(|g| g.chars().count() > 200) {
            return Err(StoreError::Invalid("the goal text is too long".into()));
        }
        Ok(())
    }

    fn map_unique(err: rusqlite::Error) -> StoreError {
        if let rusqlite::Error::SqliteFailure(f, _) = &err {
            if f.code == rusqlite::ErrorCode::ConstraintViolation {
                return StoreError::Invalid("an entry with this slug already exists".into());
            }
        }
        StoreError::Db(err)
    }

    pub fn insert_entry(&self, e: &Entry, now: i64) -> Result<i64> {
        Self::validate(e)?;
        let c = self.c();
        c.execute(
            "INSERT INTO entries(slug, name, icon_json, group_id, source_cfg_json, interval_s, format_json, goal_value, celebrate, sort, created_at, sounds_json, text_mode, goal_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                e.slug,
                e.name.trim(),
                to_json(&e.icon),
                e.group_id,
                to_json(&e.source),
                e.interval_s,
                to_json(&e.format),
                e.goal_value,
                e.celebrate as i64,
                e.sort,
                now,
                to_json(&e.sounds),
                e.text_mode as i64,
                clean_goal_text(&e.goal_text)
            ],
        )
        .map_err(Self::map_unique)?;
        Ok(c.last_insert_rowid())
    }

    pub fn update_entry(&self, e: &Entry) -> Result<()> {
        Self::validate(e)?;
        let c = self.c();
        let old: Option<(Option<f64>, Option<String>, i64)> = c
            .query_row("SELECT goal_value, goal_text, text_mode FROM entries WHERE id = ?1", [e.id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .optional()?;
        let Some((old_goal, old_goal_text, old_mode)) = old else {
            return Err(StoreError::Invalid("entry not found".into()));
        };
        let goal_text = clean_goal_text(&e.goal_text);
        c.execute(
            "UPDATE entries SET slug=?2, name=?3, icon_json=?4, group_id=?5, source_cfg_json=?6, interval_s=?7,
             format_json=?8, goal_value=?9, celebrate=?10, sort=?11, sounds_json=?12, text_mode=?13, goal_text=?14 WHERE id=?1",
            params![
                e.id,
                e.slug,
                e.name.trim(),
                to_json(&e.icon),
                e.group_id,
                to_json(&e.source),
                e.interval_s,
                to_json(&e.format),
                e.goal_value,
                e.celebrate as i64,
                e.sort,
                to_json(&e.sounds),
                e.text_mode as i64,
                goal_text
            ],
        )
        .map_err(Self::map_unique)?;
        if old_goal != e.goal_value || old_goal_text != goal_text || (old_mode != 0) != e.text_mode {
            c.execute("DELETE FROM marks WHERE entry_id = ?1 AND kind = 'goal'", [e.id])?;
        }
        Ok(())
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.c().execute("DELETE FROM entries WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn entries(&self) -> Result<Vec<Entry>> {
        let c = self.c();
        let mut st = c.prepare(&format!("SELECT {ENTRY_COLS} FROM entries ORDER BY sort, id"))?;
        let rows = st.query_map([], row_entry)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn entry(&self, id: i64) -> Result<Option<Entry>> {
        let c = self.c();
        Ok(c.query_row(&format!("SELECT {ENTRY_COLS} FROM entries WHERE id = ?1"), [id], row_entry)
            .optional()?)
    }

    pub fn entry_by_slug(&self, slug: &str) -> Result<Option<Entry>> {
        let c = self.c();
        Ok(c.query_row(&format!("SELECT {ENTRY_COLS} FROM entries WHERE slug = ?1"), [slug], row_entry)
            .optional()?)
    }

    pub fn reorder_entries(&self, ids: &[i64]) -> Result<()> {
        let mut c = self.c();
        let tx = c.transaction()?;
        for (i, id) in ids.iter().enumerate() {
            tx.execute("UPDATE entries SET sort = ?2 WHERE id = ?1", params![id, i as i64])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn groups(&self) -> Result<Vec<Group>> {
        let c = self.c();
        let mut st = c.prepare("SELECT id, name, sort FROM groups ORDER BY sort, id")?;
        let rows = st
            .query_map([], |r| Ok(Group { id: r.get(0)?, name: r.get(1)?, sort: r.get(2)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn upsert_group(&self, g: &Group) -> Result<i64> {
        if g.name.trim().is_empty() {
            return Err(StoreError::Invalid("group name is required".into()));
        }
        let c = self.c();
        if g.id > 0 {
            c.execute("UPDATE groups SET name=?2, sort=?3 WHERE id=?1", params![g.id, g.name.trim(), g.sort])?;
            Ok(g.id)
        } else {
            c.execute("INSERT INTO groups(name, sort) VALUES (?1, ?2)", params![g.name.trim(), g.sort])?;
            Ok(c.last_insert_rowid())
        }
    }

    pub fn delete_group(&self, id: i64) -> Result<()> {
        self.c().execute("DELETE FROM groups WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn alerts_for(&self, entry_id: i64) -> Result<Vec<Alert>> {
        Ok(self.alerts_with_state(entry_id)?.into_iter().map(|(a, _)| a).collect())
    }

    pub(crate) fn alerts_with_state(&self, entry_id: i64) -> Result<Vec<(Alert, bool)>> {
        let c = self.c();
        let mut st = c.prepare(
            "SELECT id, entry_id, kind, threshold, window_s, enabled, last_fired_at, armed, notify, effect FROM alerts WHERE entry_id = ?1 ORDER BY id",
        )?;
        let rows = st.query_map([entry_id], row_alert)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn upsert_alert(&self, a: &Alert) -> Result<i64> {
        if !a.threshold.is_finite() {
            return Err(StoreError::Invalid("threshold must be a number".into()));
        }
        if matches!(a.kind, AlertKind::ChangePct | AlertKind::RisePct | AlertKind::DropPct) && (a.threshold <= 0.0 || a.window_s == 0) {
            return Err(StoreError::Invalid("change alerts need a percentage above 0 and a time window".into()));
        }
        if matches!(a.kind, AlertKind::RiseBy | AlertKind::DropBy) && a.threshold <= 0.0 {
            return Err(StoreError::Invalid("the step must be above 0".into()));
        }
        if !crate::model::ALERT_EFFECTS.contains(&a.effect.as_str()) {
            return Err(StoreError::Invalid("unknown effect".into()));
        }
        let c = self.c();
        if a.id > 0 {
            c.execute(
                "UPDATE alerts SET kind=?2, threshold=?3, window_s=?4, enabled=?5, armed=1, notify=?6, effect=?7 WHERE id=?1",
                params![a.id, a.kind.as_str(), a.threshold, a.window_s, a.enabled as i64, a.notify as i64, a.effect],
            )?;
            Ok(a.id)
        } else {
            c.execute(
                "INSERT INTO alerts(entry_id, kind, threshold, window_s, enabled, notify, effect) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![a.entry_id, a.kind.as_str(), a.threshold, a.window_s, a.enabled as i64, a.notify as i64, a.effect],
            )?;
            Ok(c.last_insert_rowid())
        }
    }

    pub fn delete_alert(&self, id: i64) -> Result<()> {
        self.c().execute("DELETE FROM alerts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub(crate) fn set_alert_state(&self, id: i64, armed: bool, fired_at: Option<i64>) -> Result<()> {
        let c = self.c();
        match fired_at {
            Some(ts) => c.execute("UPDATE alerts SET armed=?2, last_fired_at=?3 WHERE id=?1", params![id, armed as i64, ts])?,
            None => c.execute("UPDATE alerts SET armed=?2 WHERE id=?1", params![id, armed as i64])?,
        };
        Ok(())
    }

    pub fn mark(&self, entry_id: i64, kind: &str) -> Result<Option<i64>> {
        let c = self.c();
        Ok(c.query_row("SELECT ts FROM marks WHERE entry_id=?1 AND kind=?2", params![entry_id, kind], |r| r.get(0))
            .optional()?)
    }

    pub fn set_mark(&self, entry_id: i64, kind: &str, ts: i64) -> Result<()> {
        self.c().execute(
            "INSERT INTO marks(entry_id, kind, ts) VALUES (?1, ?2, ?3) ON CONFLICT(entry_id, kind) DO UPDATE SET ts = excluded.ts",
            params![entry_id, kind, ts],
        )?;
        Ok(())
    }

    pub fn clear_mark(&self, entry_id: i64, kind: &str) -> Result<()> {
        self.c().execute("DELETE FROM marks WHERE entry_id=?1 AND kind=?2", params![entry_id, kind])?;
        Ok(())
    }

    pub fn insert_reading(&self, r: &Reading) -> Result<()> {
        let value = r.value.filter(|v| v.is_finite());
        self.c().execute(
            "INSERT INTO readings(entry_id, ts, value, status, error, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![r.entry_id, r.ts, value, r.status.as_str(), r.error, r.text],
        )?;
        Ok(())
    }

    pub fn latest(&self, entry_id: i64) -> Result<Option<Reading>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT entry_id, ts, value, status, error, text FROM readings WHERE entry_id=?1 ORDER BY ts DESC, rowid DESC LIMIT 1",
            [entry_id],
            row_reading,
        )
        .optional()?)
    }

    /// The last two texts recorded for a text entry, newest first.
    pub fn last_texts(&self, entry_id: i64) -> Result<(Option<String>, Option<String>)> {
        let c = self.c();
        let mut st = c.prepare("SELECT text FROM readings WHERE entry_id=?1 AND text IS NOT NULL ORDER BY ts DESC, rowid DESC LIMIT 2")?;
        let mut it = st.query_map([entry_id], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter();
        Ok((it.next(), it.next()))
    }

    pub fn latest_value(&self, entry_id: i64) -> Result<Option<(i64, f64)>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT ts, value FROM readings WHERE entry_id=?1 AND value IS NOT NULL ORDER BY ts DESC, rowid DESC LIMIT 1",
            [entry_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
    }

    /// The value recorded before the latest value.
    pub fn previous_value(&self, entry_id: i64) -> Result<Option<f64>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT value FROM readings WHERE entry_id=?1 AND value IS NOT NULL ORDER BY ts DESC, rowid DESC LIMIT 1 OFFSET 1",
            [entry_id],
            |r| r.get(0),
        )
        .optional()?)
    }

    pub fn value_at_or_before(&self, entry_id: i64, ts: i64) -> Result<Option<f64>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT value FROM readings WHERE entry_id=?1 AND value IS NOT NULL AND ts <= ?2 ORDER BY ts DESC, rowid DESC LIMIT 1",
            params![entry_id, ts],
            |r| r.get(0),
        )
        .optional()?)
    }

    pub fn first_value_since(&self, entry_id: i64, ts: i64) -> Result<Option<f64>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT value FROM readings WHERE entry_id=?1 AND value IS NOT NULL AND ts >= ?2 ORDER BY ts ASC, rowid ASC LIMIT 1",
            params![entry_id, ts],
            |r| r.get(0),
        )
        .optional()?)
    }

    /// Highest value recorded strictly before `ts`.
    pub fn max_value_before(&self, entry_id: i64, ts: i64) -> Result<Option<f64>> {
        let c = self.c();
        Ok(c.query_row(
            "SELECT MAX(value) FROM readings WHERE entry_id=?1 AND value IS NOT NULL AND ts < ?2",
            params![entry_id, ts],
            |r| r.get(0),
        )?)
    }

    pub fn history(&self, entry_id: i64, since: i64, until: i64) -> Result<Vec<(i64, f64)>> {
        let c = self.c();
        let mut st = c.prepare(
            "SELECT ts, value FROM readings WHERE entry_id=?1 AND value IS NOT NULL AND ts >= ?2 AND ts <= ?3 ORDER BY ts",
        )?;
        let rows = st
            .query_map(params![entry_id, since, until], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Bucket averages over `[since, until]`; empty buckets are skipped.
    pub fn spark(&self, entry_id: i64, since: i64, until: i64, points: usize) -> Result<Vec<(i64, f64)>> {
        if points == 0 || until <= since {
            return Ok(vec![]);
        }
        let width = ((until - since) as f64 / points as f64).ceil().max(1.0) as i64;
        let c = self.c();
        let mut st = c.prepare(
            "SELECT ?2 + ((ts - ?2) / ?4) * ?4 AS b, AVG(value) FROM readings
             WHERE entry_id=?1 AND value IS NOT NULL AND ts >= ?2 AND ts <= ?3 GROUP BY b ORDER BY b",
        )?;
        let rows = st
            .query_map(params![entry_id, since, until, width], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Retention: full resolution for 30 days, hourly averages up to a year, daily beyond.
    pub fn compact(&self, now: i64) -> Result<()> {
        let cut30 = ((now - 30 * DAY_MS) / HOUR_MS) * HOUR_MS;
        let cut365 = ((now - 365 * DAY_MS) / DAY_MS) * DAY_MS;
        let mut c = self.c();
        let tx = c.transaction()?;
        tx.execute("DELETE FROM readings WHERE ts < ?1 AND value IS NULL", [cut30])?;
        for (lo, hi, width) in [(cut365, cut30, HOUR_MS), (i64::MIN, cut365, DAY_MS)] {
            tx.execute_batch("DROP TABLE IF EXISTS temp.agg")?;
            tx.execute(
                "CREATE TEMP TABLE agg AS
                 SELECT entry_id, (ts / ?3) * ?3 AS b, AVG(value) AS v FROM readings
                 WHERE ts >= ?1 AND ts < ?2 GROUP BY entry_id, b HAVING COUNT(*) > 1 OR MIN(ts) != b",
                params![lo, hi, width],
            )?;
            tx.execute(
                "DELETE FROM readings WHERE ts >= ?1 AND ts < ?2 AND EXISTS
                 (SELECT 1 FROM agg WHERE agg.entry_id = readings.entry_id AND agg.b = (readings.ts / ?3) * ?3)",
                params![lo, hi, width],
            )?;
            tx.execute("INSERT INTO readings(entry_id, ts, value, status, error) SELECT entry_id, b, v, 'live', NULL FROM agg", [])?;
            tx.execute_batch("DROP TABLE temp.agg")?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        let c = self.c();
        Ok(c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.c().execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<()> {
        self.c().execute("DELETE FROM settings WHERE key=?1", [key])?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn raw_source_json(&self, id: i64) -> String {
        self.c().query_row("SELECT source_cfg_json FROM entries WHERE id=?1", [id], |r| r.get(0)).unwrap()
    }

    #[cfg(test)]
    pub(crate) fn reading_count(&self, entry_id: i64) -> i64 {
        self.c().query_row("SELECT COUNT(*) FROM readings WHERE entry_id=?1", [entry_id], |r| r.get(0)).unwrap()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn entry(slug: &str, source: SourceCfg) -> Entry {
        Entry {
            id: 0,
            slug: slug.into(),
            name: slug.into(),
            icon: Icon::default(),
            group_id: None,
            source,
            interval_s: 60,
            format: NumberFormat::default(),
            goal_value: None,
            celebrate: true,
            sounds: Sounds::default(),
            sort: 0,
            text_mode: false,
            goal_text: None,
        }
    }

    pub fn url_source() -> SourceCfg {
        SourceCfg::CustomUrl {
            url: "https://example.com".into(),
            method: "GET".into(),
            headers: vec![],
            body: None,
            pick: Pick::FirstNumber,
        }
    }

    fn reading(id: i64, ts: i64, v: Option<f64>, s: Status) -> Reading {
        Reading { entry_id: id, ts, value: v, status: s, error: None, text: None }
    }

    #[test]
    fn rejects_bad_entries() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("orders", url_source());
        e.interval_s = 5;
        assert!(matches!(s.insert_entry(&e, 0), Err(StoreError::Invalid(_))));
        let mut e = entry("Bad Slug", url_source());
        e.interval_s = 60;
        assert!(s.insert_entry(&e, 0).is_err());
        s.insert_entry(&entry("orders", url_source()), 0).unwrap();
        let err = s.insert_entry(&entry("orders", url_source()), 0).unwrap_err();
        assert_eq!(err.to_string(), "an entry with this slug already exists");
    }

    #[test]
    fn latest_value_skips_errors() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.insert_reading(&reading(id, 1000, Some(10.0), Status::Live)).unwrap();
        s.insert_reading(&reading(id, 2000, None, Status::Error)).unwrap();
        assert_eq!(s.latest_value(id).unwrap(), Some((1000, 10.0)));
        assert_eq!(s.latest(id).unwrap().unwrap().status, Status::Error);
    }

    #[test]
    fn delete_cascades() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        s.insert_reading(&reading(id, 1, Some(1.0), Status::Live)).unwrap();
        s.upsert_alert(&Alert {
            id: 0,
            entry_id: id,
            kind: AlertKind::Above,
            threshold: 5.0,
            window_s: 60,
            enabled: true,
            last_fired_at: None,
            notify: true,
            effect: String::new(),
        })
        .unwrap();
        s.delete_entry(id).unwrap();
        assert!(s.latest(id).unwrap().is_none());
        assert!(s.alerts_for(id).unwrap().is_empty());
    }

    #[test]
    fn compact_downsamples() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        let now = 1_800_000_000_000i64;
        let start = ((now - 40 * DAY_MS) / HOUR_MS) * HOUR_MS;
        for i in 0..60 {
            s.insert_reading(&reading(id, start + i * 60_000, Some(i as f64), Status::Live)).unwrap();
        }
        s.insert_reading(&reading(id, start + 5, None, Status::Error)).unwrap();
        let old = ((now - 400 * DAY_MS) / DAY_MS) * DAY_MS;
        for i in 0..4 {
            s.insert_reading(&reading(id, old + i * HOUR_MS, Some(10.0 * i as f64), Status::Live)).unwrap();
        }
        s.insert_reading(&reading(id, now - DAY_MS, Some(7.0), Status::Live)).unwrap();
        s.insert_reading(&reading(id, now - DAY_MS + 1, Some(8.0), Status::Live)).unwrap();
        s.compact(now).unwrap();
        assert_eq!(s.history(id, start, start + HOUR_MS - 1).unwrap(), vec![(start, 29.5)]);
        assert_eq!(s.history(id, old, old + DAY_MS - 1).unwrap(), vec![(old, 15.0)]);
        assert_eq!(s.history(id, now - 2 * DAY_MS, now).unwrap().len(), 2);
        assert_eq!(s.reading_count(id), 4);
        s.compact(now).unwrap();
        assert_eq!(s.reading_count(id), 4);
    }

    #[test]
    fn spark_buckets() {
        let s = Store::open_memory().unwrap();
        let id = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        for i in 0..100 {
            s.insert_reading(&reading(id, i * 1000, Some(i as f64), Status::Live)).unwrap();
        }
        let sp = s.spark(id, 0, 99_999, 10).unwrap();
        assert_eq!(sp.len(), 10);
        assert!(sp.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(sp[0].1, 4.5);
    }

    #[test]
    fn secrets_never_stored() {
        let s = Store::open_memory().unwrap();
        let src = SourceCfg::CustomUrl {
            url: "https://api.example.com".into(),
            method: "GET".into(),
            headers: vec![Header { name: "Authorization".into(), value: SecretOrText::Secret("entry-1-h0".into()) }],
            body: None,
            pick: Pick::JsonPath("count".into()),
        };
        let id = s.insert_entry(&entry("a", src.clone()), 0).unwrap();
        let raw = s.raw_source_json(id);
        assert!(raw.contains(r#""kind":"secret","value":"entry-1-h0""#));
        assert_eq!(s.entry(id).unwrap().unwrap().source, src);
    }

    #[test]
    fn goal_change_clears_mark() {
        let s = Store::open_memory().unwrap();
        let mut e = entry("a", url_source());
        e.goal_value = Some(10.0);
        e.id = s.insert_entry(&e, 0).unwrap();
        s.set_mark(e.id, "goal", 5).unwrap();
        s.update_entry(&e).unwrap();
        assert_eq!(s.mark(e.id, "goal").unwrap(), Some(5));
        e.goal_value = Some(20.0);
        s.update_entry(&e).unwrap();
        assert_eq!(s.mark(e.id, "goal").unwrap(), None);
    }

    #[test]
    fn opens_file_db() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub/cownter.db");
        let s = Store::open(&p).unwrap();
        s.insert_entry(&entry("a", url_source()), 0).unwrap();
        drop(s);
        let s = Store::open(&p).unwrap();
        assert_eq!(s.entries().unwrap().len(), 1);
    }

    #[test]
    fn migrates_old_alert_table() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE alerts(id INTEGER PRIMARY KEY, entry_id INTEGER NOT NULL, kind TEXT NOT NULL, threshold REAL NOT NULL, window_s INTEGER NOT NULL DEFAULT 3600, enabled INTEGER NOT NULL DEFAULT 1, last_fired_at INTEGER, armed INTEGER NOT NULL DEFAULT 1);",
        )
        .unwrap();
        let s = Store::init(c).unwrap();
        let id = s.insert_entry(&entry("m", url_source()), 0).unwrap();
        let a = Alert { id: 0, entry_id: id, kind: AlertKind::Above, threshold: 1.0, window_s: 60, enabled: true, last_fired_at: None, notify: false, effect: "burst".into() };
        s.upsert_alert(&a).unwrap();
        let got = &s.alerts_for(id).unwrap()[0];
        assert!(!got.notify);
        assert_eq!(got.effect, "burst");
    }
}
