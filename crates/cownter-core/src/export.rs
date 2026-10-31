use crate::model::Entry;
use crate::store::{Result, Store};
use chrono::{Local, SecondsFormat, TimeZone};

pub const HEADER: &str = "entry,timestamp,value";

/// Quotes a field when it holds a comma, a quote or a line break.
pub fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

fn number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v}") }
}

/// One line per reading with an ISO-8601 timestamp in `tz`, after a header line.
pub fn csv<'a, Tz: TimeZone>(rows: impl IntoIterator<Item = (&'a str, i64, f64)>, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let mut out = String::from(HEADER);
    out.push('\n');
    for (name, ts, value) in rows {
        let Some(t) = tz.timestamp_millis_opt(ts).single() else { continue };
        out.push_str(&format!("{},{},{}\n", field(name), t.to_rfc3339_opts(SecondsFormat::Secs, false), number(value)));
    }
    out
}

/// The full stored history of `entries`, in local time.
pub fn entries_csv(store: &Store, entries: &[Entry]) -> Result<String> {
    let mut rows = Vec::new();
    for e in entries {
        for (ts, v) in store.history(e.id, i64::MIN, i64::MAX)? {
            rows.push((e.name.as_str(), ts, v));
        }
    }
    Ok(csv(rows, &Local))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Reading, Status};
    use crate::store::tests::{entry, url_source};
    use chrono::FixedOffset;

    #[test]
    fn escapes_fields() {
        assert_eq!(field("Orders"), "Orders");
        assert_eq!(field("Orders, EU"), "\"Orders, EU\"");
        assert_eq!(field("The \"big\" one"), "\"The \"\"big\"\" one\"");
        assert_eq!(field("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn formats_rows() {
        let tz = FixedOffset::east_opt(3600).unwrap();
        let out = csv([("Sales, \"EU\"", 1_793_000_000_000, 42.0), ("Rate", 1_793_000_060_500, 0.25)], &tz);
        assert_eq!(
            out,
            "entry,timestamp,value\n\"Sales, \"\"EU\"\"\",2026-10-26T08:33:20+01:00,42\nRate,2026-10-26T08:34:20+01:00,0.25\n"
        );
        assert_eq!(csv([], &tz), "entry,timestamp,value\n");
    }

    #[test]
    fn exports_stored_history() {
        let s = Store::open_memory().unwrap();
        let a = s.insert_entry(&entry("a", url_source()), 0).unwrap();
        let b = s.insert_entry(&entry("b", url_source()), 0).unwrap();
        for (id, ts, v) in [(a, 1000, Some(1.0)), (b, 1500, Some(9.0)), (a, 2000, None), (a, 3000, Some(2.5))] {
            let status = if v.is_some() { Status::Live } else { Status::Error };
            s.insert_reading(&Reading { entry_id: id, ts, value: v, status, error: None, text: None }).unwrap();
        }
        let all = s.entries().unwrap();
        let out = entries_csv(&s, &all).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[1].starts_with("a,") && lines[1].ends_with(",1"));
        assert!(lines[2].starts_with("a,") && lines[2].ends_with(",2.5"));
        assert!(lines[3].starts_with("b,") && lines[3].ends_with(",9"));
        let one = entries_csv(&s, &all[1..]).unwrap();
        assert_eq!(one.lines().count(), 2);
    }
}
