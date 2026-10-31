use crate::display::DisplayConfig;
use cownter_core::push::{self, ClientError};
use cownter_core::secrets::KeyringSecrets;
use cownter_core::store::Store;
use serde_json::{Value, json};

/// Same as the app identifier in tauri.conf.json; names the data folder.
const IDENTIFIER: &str = "app.cownter.desktop";

const HELP: &str = "Cownter command line

Usage:
  cownter set <entry> <value>   Set a Push number (or text) to a value
  cownter add <entry> [amount]  Add to a Push number (default 1, can be negative)
  cownter get <entry>           Print the current value of any number
  cownter list                  List every number with its value
  cownter export [entry] [--out <file>]
                                Print the history of one number (or all) as CSV,
                                or write it to <file>
  cownter --help                Show this help

<entry> is the number's name, short name or id.
Cownter must be running; commands talk to it on 127.0.0.1.

Exit codes: 0 ok, 1 error.";

#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Help,
    Set { entry: String, value: String },
    Add { entry: String, by: f64 },
    Get { entry: String },
    List,
    Export { entry: Option<String>, out: Option<String> },
}

const USAGE: &str = "run `cownter --help` for usage";

/// Parses the arguments after the program name; `None` means it is not a CLI call.
pub fn parse(args: &[String]) -> Option<Result<Cmd, String>> {
    let first = args.first()?.as_str();
    let rest = &args[1..];
    let wrong = |u: &str| Err(format!("usage: cownter {u}"));
    Some(match first {
        "--help" | "-h" | "help" => Ok(Cmd::Help),
        "set" => match rest {
            [entry, value] => Ok(Cmd::Set { entry: entry.clone(), value: value.clone() }),
            _ => wrong("set <entry> <value>"),
        },
        "add" => match rest {
            [entry] => Ok(Cmd::Add { entry: entry.clone(), by: 1.0 }),
            [entry, by] => match by.trim().parse::<f64>() {
                Ok(b) if b.is_finite() => Ok(Cmd::Add { entry: entry.clone(), by: b }),
                _ => Err(format!("`{by}` is not a number")),
            },
            _ => wrong("add <entry> [amount]"),
        },
        "get" => match rest {
            [entry] => Ok(Cmd::Get { entry: entry.clone() }),
            _ => wrong("get <entry>"),
        },
        "list" => match rest {
            [] => Ok(Cmd::List),
            _ => wrong("list"),
        },
        "export" => parse_export(rest),
        _ => return None,
    })
}

fn parse_export(rest: &[String]) -> Result<Cmd, String> {
    let usage = || Err("usage: cownter export [entry] [--out <file>]".to_string());
    let (mut entry, mut out) = (None, None);
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "--out" || a == "-o" {
            match it.next() {
                Some(p) if out.is_none() => out = Some(p.clone()),
                _ => return usage(),
            }
        } else if let Some(p) = a.strip_prefix("--out=") {
            if out.is_some() || p.is_empty() {
                return usage();
            }
            out = Some(p.to_string());
        } else if entry.is_none() && !a.starts_with('-') {
            entry = Some(a.clone());
        } else {
            return usage();
        }
    }
    Ok(Cmd::Export { entry, out })
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{v}") }
}

/// The value of an entry as printed: its text, its number or `-` when it has none yet.
pub fn shown(e: &Value) -> String {
    if let Some(t) = e["text"].as_str() {
        return t.to_string();
    }
    e["value"].as_f64().map(fmt_num).unwrap_or_else(|| "-".into())
}

pub fn table(entries: &[Value]) -> String {
    if entries.is_empty() {
        return "No numbers yet.".into();
    }
    let rows: Vec<(String, String)> = entries
        .iter()
        .map(|e| (format!("{} ({})", e["name"].as_str().unwrap_or(""), e["slug"].as_str().unwrap_or("")), shown(e)))
        .collect();
    let w = rows.iter().map(|(n, _)| n.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|(n, v)| format!("{n}{}  {v}", " ".repeat(w - n.chars().count())))
        .collect::<Vec<_>>()
        .join("\n")
}

fn connection() -> Result<(u16, String), String> {
    let dir = dirs::data_dir().ok_or("could not find the data folder")?.join(IDENTIFIER);
    let db = dir.join("cownter.db");
    if !db.exists() {
        return Err("Cownter isn't set up yet; open the app once first".into());
    }
    let store = Store::open(&db).map_err(|e| e.to_string())?;
    let port = DisplayConfig::load(&store).push_port;
    let token = push::read_token(&KeyringSecrets::new(), &store)
        .ok_or("no push token found; start Cownter once to create one")?;
    Ok((port, token))
}

fn request(method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let (port, token) = connection()?;
    let (status, out) = push::call(port, &token, method, path, body.as_ref()).map_err(|e| match e {
        ClientError::NotRunning => "Cownter isn't running".to_string(),
        other => other.to_string(),
    })?;
    if status == 200 {
        Ok(out)
    } else {
        Err(out["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("Cownter answered HTTP {status}")))
    }
}

fn request_csv(path: &str) -> Result<String, String> {
    let (port, token) = connection()?;
    let (status, out) = push::call_text(port, &token, path).map_err(|e| match e {
        ClientError::NotRunning => "Cownter isn't running".to_string(),
        other => other.to_string(),
    })?;
    if status == 200 {
        return Ok(out);
    }
    let err = serde_json::from_str::<Value>(&out).ok().and_then(|v| v["error"].as_str().map(str::to_string));
    Err(err.unwrap_or_else(|| format!("Cownter answered HTTP {status}")))
}

fn exec(cmd: Cmd) -> Result<String, String> {
    match cmd {
        Cmd::Help => Ok(HELP.into()),
        Cmd::Set { entry, value } => request("POST", "/v1/set", Some(json!({ "entry": entry, "value": value }))).map(|e| shown(&e)),
        Cmd::Add { entry, by } => request("POST", "/v1/add", Some(json!({ "entry": entry, "by": by }))).map(|e| shown(&e)),
        Cmd::Get { entry } => {
            let e = request("GET", &format!("/v1/get?entry={}", push::encode_query(&entry)), None)?;
            if e["value"].is_null() && e["text"].is_null() {
                return Err(format!("`{}` has no value yet", e["name"].as_str().unwrap_or(&entry)));
            }
            Ok(shown(&e))
        }
        Cmd::List => {
            let out = request("GET", "/v1/list", None)?;
            Ok(table(out["entries"].as_array().map(Vec::as_slice).unwrap_or(&[])))
        }
        Cmd::Export { entry, out } => {
            let path = match &entry {
                Some(e) => format!("/v1/export?entry={}", push::encode_query(e)),
                None => "/v1/export".into(),
            };
            let csv = request_csv(&path)?;
            match out {
                Some(file) => {
                    std::fs::write(&file, csv).map_err(|e| format!("could not write {file}: {e}"))?;
                    Ok(format!("Saved to {file}"))
                }
                None => Ok(csv.trim_end_matches('\n').to_string()),
            }
        }
    }
}

/// Runs a CLI command when the arguments name one and returns the exit code.
pub fn run(args: &[String]) -> Option<i32> {
    let res = match parse(args)? {
        Ok(cmd) => exec(cmd),
        Err(e) => Err(format!("{e}\n{USAGE}")),
    };
    Some(match res {
        Ok(out) => {
            println!("{out}");
            0
        }
        Err(e) => {
            eprintln!("cownter: {e}");
            1
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Option<Result<Cmd, String>> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_commands() {
        assert_eq!(p(&["set", "Orders", "42"]), Some(Ok(Cmd::Set { entry: "Orders".into(), value: "42".into() })));
        assert_eq!(p(&["add", "Orders"]), Some(Ok(Cmd::Add { entry: "Orders".into(), by: 1.0 })));
        assert_eq!(p(&["add", "Orders", "-2.5"]), Some(Ok(Cmd::Add { entry: "Orders".into(), by: -2.5 })));
        assert_eq!(p(&["get", "Orders today"]), Some(Ok(Cmd::Get { entry: "Orders today".into() })));
        assert_eq!(p(&["list"]), Some(Ok(Cmd::List)));
        assert_eq!(p(&["--help"]), Some(Ok(Cmd::Help)));
        assert_eq!(p(&["export"]), Some(Ok(Cmd::Export { entry: None, out: None })));
        assert_eq!(p(&["export", "Orders"]), Some(Ok(Cmd::Export { entry: Some("Orders".into()), out: None })));
        assert_eq!(
            p(&["export", "--out", "a.csv", "Orders"]),
            Some(Ok(Cmd::Export { entry: Some("Orders".into()), out: Some("a.csv".into()) }))
        );
        assert_eq!(p(&["export", "--out=b.csv"]), Some(Ok(Cmd::Export { entry: None, out: Some("b.csv".into()) })));
    }

    #[test]
    fn leaves_other_launches_to_the_app() {
        assert_eq!(p(&[]), None);
        assert_eq!(p(&["--autostart"]), None);
        assert_eq!(p(&["-psn_0_12345"]), None);
    }

    #[test]
    fn reports_bad_usage() {
        assert!(p(&["set", "Orders"]).unwrap().is_err());
        assert!(p(&["add", "Orders", "lots"]).unwrap().unwrap_err().contains("not a number"));
        assert!(p(&["get"]).unwrap().is_err());
        assert!(p(&["list", "x"]).unwrap().is_err());
        assert!(p(&["export", "--out"]).unwrap().is_err());
        assert!(p(&["export", "a", "b"]).unwrap().is_err());
        assert!(p(&["export", "--json"]).unwrap().is_err());
    }

    #[test]
    fn formats_values() {
        assert_eq!(shown(&json!({"value": 42.0, "text": null})), "42");
        assert_eq!(shown(&json!({"value": 2.5})), "2.5");
        assert_eq!(shown(&json!({"value": null, "text": "green"})), "green");
        assert_eq!(shown(&json!({"value": null})), "-");
        let t = table(&[json!({"name": "Orders", "slug": "orders", "value": 3.0}), json!({"name": "Stars", "slug": "gh", "value": null})]);
        assert_eq!(t, "Orders (orders)  3\nStars (gh)       -");
        assert_eq!(table(&[]), "No numbers yet.");
    }
}
