use crate::model::Pick;
use serde_json::Value;

enum Seg {
    Key(String),
    Index(usize),
}

fn tokenize(path: &str) -> Option<Vec<Seg>> {
    let mut out = Vec::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;
    let mut key = String::new();
    let flush = |key: &mut String, out: &mut Vec<Seg>| {
        if !key.is_empty() {
            match key.parse::<usize>() {
                Ok(n) => out.push(Seg::Index(n)),
                Err(_) => out.push(Seg::Key(std::mem::take(key))),
            }
            key.clear();
        }
    };
    while i < chars.len() {
        match chars[i] {
            '.' => {
                flush(&mut key, &mut out);
                i += 1;
            }
            '[' => {
                flush(&mut key, &mut out);
                i += 1;
                if i < chars.len() && chars[i] == '"' {
                    i += 1;
                    let mut k = String::new();
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' && i + 1 < chars.len() {
                            i += 1;
                        }
                        k.push(chars[i]);
                        i += 1;
                    }
                    if i + 1 >= chars.len() || chars[i + 1] != ']' {
                        return None;
                    }
                    i += 2;
                    out.push(Seg::Key(k));
                } else {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i == start || i >= chars.len() || chars[i] != ']' {
                        return None;
                    }
                    let n: String = chars[start..i].iter().collect();
                    out.push(Seg::Index(n.parse().ok()?));
                    i += 1;
                }
            }
            c => {
                key.push(c);
                i += 1;
            }
        }
    }
    flush(&mut key, &mut out);
    Some(out)
}

/// Selects a value by a path like `data.stats.subs`, `items[0].count` or `["a.b"]`.
/// An empty path selects the root.
pub fn pick_path<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let segs = tokenize(path.trim())?;
    let mut cur = v;
    for s in segs {
        cur = match (s, cur) {
            (Seg::Key(k), Value::Object(m)) => m.get(&k)?,
            (Seg::Index(n), Value::Array(a)) => a.get(n)?,
            (Seg::Index(n), Value::Object(m)) => m.get(&n.to_string())?,
            _ => return None,
        };
    }
    Some(cur)
}

pub fn number_from_value(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|f| f.is_finite()),
        Value::String(s) => parse_number(s),
        _ => None,
    }
}

fn is_group_sep(c: char) -> bool {
    matches!(c, ' ' | '\'' | '\u{00a0}' | '\u{202f}' | '\u{2009}')
}

fn normalize_digits(raw: &str) -> Option<f64> {
    let s: String = raw.chars().filter(|c| !is_group_sep(*c)).collect();
    let dots = s.matches('.').count();
    let commas = s.matches(',').count();
    let cleaned = if dots > 0 && commas > 0 {
        let last_dot = s.rfind('.').unwrap();
        let last_comma = s.rfind(',').unwrap();
        let (dec, thou) = if last_dot > last_comma { ('.', ',') } else { (',', '.') };
        if s.matches(dec).count() > 1 {
            return None;
        }
        s.replace(thou, "").replace(dec, ".")
    } else if dots + commas == 0 {
        s
    } else {
        let sep = if dots > 0 { '.' } else { ',' };
        let count = dots + commas;
        if count > 1 {
            s.replace(sep, "")
        } else {
            let after = s.len() - s.find(sep).unwrap() - 1;
            if sep == ',' && after == 3 { s.replace(',', "") } else { s.replace(sep, ".") }
        }
    };
    if cleaned.is_empty() || cleaned.starts_with('.') || cleaned.ends_with('.') {
        return None;
    }
    cleaned.parse::<f64>().ok().filter(|f| f.is_finite())
}

fn multiplier(c: char) -> Option<f64> {
    match c.to_ascii_lowercase() {
        'k' => Some(1e3),
        'm' => Some(1e6),
        'b' => Some(1e9),
        _ => None,
    }
}

/// Parses a string holding exactly one number, e.g. `"1,284"`, `"€2.9K"`, `"1.284,50"`, `"12%"`.
pub fn parse_number(s: &str) -> Option<f64> {
    let s = s.trim();
    let first_digit = s.find(|c: char| c.is_ascii_digit())?;
    let prefix = &s[..first_digit];
    let negative = prefix.contains('-') || prefix.contains('−');
    let letters: Vec<char> = prefix.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 3 || letters.iter().any(|c| !c.is_uppercase()) {
        return None;
    }
    let rest = &s[first_digit..];
    let mut end = 0;
    let rc: Vec<(usize, char)> = rest.char_indices().collect();
    for (k, &(i, c)) in rc.iter().enumerate() {
        let ok = c.is_ascii_digit()
            || ((c == '.' || c == ',' || is_group_sep(c))
                && rc.get(k + 1).is_some_and(|(_, n)| n.is_ascii_digit()));
        if !ok {
            break;
        }
        end = i + c.len_utf8();
    }
    let num = &rest[..end];
    let tail = rest[end..].trim_start();
    let mut tail_chars = tail.chars();
    let (mult, tail) = match tail_chars.next() {
        Some(c) if multiplier(c).is_some() && !tail_chars.next().is_some_and(|n| n.is_alphabetic()) => {
            (multiplier(c).unwrap(), &tail[c.len_utf8()..])
        }
        _ => (1.0, tail),
    };
    if tail.chars().any(|c| c.is_ascii_digit() || c.is_alphabetic()) {
        if !tail.trim().chars().all(|c| c.is_ascii_uppercase()) || tail.trim().len() > 3 {
            return None;
        }
    }
    let v = normalize_digits(num)? * mult;
    Some(if negative { -v } else { v })
}

/// Returns the first number found anywhere in `text`.
pub fn first_number(text: &str) -> Option<f64> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let start_k = chars.iter().position(|(_, c)| c.is_ascii_digit())?;
    let negative = start_k > 0 && matches!(chars[start_k - 1].1, '-' | '−');
    let mut end_k = start_k;
    while end_k + 1 < chars.len() {
        let c = chars[end_k + 1].1;
        let next_digit = chars.get(end_k + 2).is_some_and(|(_, n)| n.is_ascii_digit());
        if c.is_ascii_digit() || ((c == '.' || c == ',') && next_digit) {
            end_k += 1;
        } else {
            break;
        }
    }
    let from = chars[start_k].0;
    let to = chars[end_k].0 + chars[end_k].1.len_utf8();
    let mut v = normalize_digits(&text[from..to])?;
    if let Some((_, c)) = chars.get(end_k + 1) {
        let after = chars.get(end_k + 2).map(|(_, c)| *c);
        if let Some(m) = multiplier(*c) {
            if !after.is_some_and(|a| a.is_alphabetic()) {
                v *= m;
            }
        }
    }
    Some(if negative { -v } else { v })
}

pub fn extract(body: &str, pick: &Pick) -> Result<f64, String> {
    match pick {
        Pick::JsonPath(path) => {
            let v: Value = serde_json::from_str(body).map_err(|_| "response is not JSON".to_string())?;
            let found = pick_path(&v, path).ok_or_else(|| format!("`{path}` not found in response"))?;
            number_from_value(found).ok_or_else(|| format!("`{path}` is not a number"))
        }
        Pick::FirstNumber => first_number(body).ok_or_else(|| "no number found in response".to_string()),
        Pick::Regex(pattern) => {
            let re = regex::Regex::new(pattern).map_err(|e| format!("invalid pattern: {e}"))?;
            let caps = re.captures(body).ok_or_else(|| "pattern did not match the response".to_string())?;
            let m = caps.get(1).or_else(|| caps.get(0)).unwrap();
            parse_number(m.as_str()).ok_or_else(|| format!("matched text `{}` is not a number", m.as_str()))
        }
    }
}

const MAX_TEXT: usize = 200;

fn clip_text(t: &str) -> Option<String> {
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        return None;
    }
    Some(t.chars().take(MAX_TEXT).collect())
}

/// Picks a short text from a response: a JSON value, a regex group, or the first non-empty line.
pub fn extract_text(body: &str, pick: &Pick) -> Result<String, String> {
    let found = match pick {
        Pick::JsonPath(path) => {
            let v: Value = serde_json::from_str(body).map_err(|_| "response is not JSON".to_string())?;
            let found = pick_path(&v, path).ok_or_else(|| format!("`{path}` not found in response"))?;
            match found {
                Value::String(s) => s.clone(),
                Value::Null => return Err(format!("`{path}` is empty")),
                Value::Bool(_) | Value::Number(_) => found.to_string(),
                other => other.to_string(),
            }
        }
        Pick::FirstNumber => body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string(),
        Pick::Regex(pattern) => {
            let re = regex::Regex::new(pattern).map_err(|e| format!("invalid pattern: {e}"))?;
            let caps = re.captures(body).ok_or_else(|| "pattern did not match the response".to_string())?;
            caps.get(1).or_else(|| caps.get(0)).unwrap().as_str().to_string()
        }
    };
    clip_text(&found).ok_or_else(|| "the picked text is empty".to_string())
}

/// Same text comparison the goal uses: trimmed and case-insensitive.
pub fn same_text(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text() {
        let body = r#"{"status":"  Shipped\n today ","n":3,"ok":true,"none":null}"#;
        assert_eq!(extract_text(body, &Pick::JsonPath("status".into())).unwrap(), "Shipped today");
        assert_eq!(extract_text(body, &Pick::JsonPath("n".into())).unwrap(), "3");
        assert_eq!(extract_text(body, &Pick::JsonPath("ok".into())).unwrap(), "true");
        assert!(extract_text(body, &Pick::JsonPath("none".into())).is_err());
        assert_eq!(extract_text("\n\n  hello world \nsecond", &Pick::FirstNumber).unwrap(), "hello world");
        assert_eq!(extract_text("state: OPEN;", &Pick::Regex("state: (\\w+)".into())).unwrap(), "OPEN");
        assert_eq!(extract_text(&"x".repeat(500), &Pick::FirstNumber).unwrap().len(), 200);
        assert!(extract_text("   ", &Pick::FirstNumber).is_err());
        assert!(same_text(" Shipped ", "shipped"));
    }

    #[test]
    fn parses_formats() {
        for (s, n) in [
            ("1,284", 1284.0),
            ("1.284,50", 1284.5),
            ("2,940.75", 2940.75),
            ("€2.9K", 2900.0),
            ("1.5M", 1_500_000.0),
            ("-12", -12.0),
            ("12%", 12.0),
            ("48213", 48213.0),
            ("1.000.000", 1_000_000.0),
            ("3.14", 3.14),
            ("48 213", 48213.0),
            ("USD 12.50", 12.5),
            ("12 EUR", 12.0),
            ("$ -3", -3.0),
            ("0", 0.0),
        ] {
            assert_eq!(parse_number(s), Some(n), "{s}");
        }
        for s in ["abc", "", "subs 12", "12 of 30", "1.2.3,4,5"] {
            assert_eq!(parse_number(s), None, "{s}");
        }
    }

    #[test]
    fn picks_paths() {
        let v = serde_json::json!({"data":{"stats":{"subs":"48213"}},"items":[{"count":37}],"a.b":5});
        assert_eq!(number_from_value(pick_path(&v, "data.stats.subs").unwrap()), Some(48213.0));
        assert_eq!(number_from_value(pick_path(&v, "items[0].count").unwrap()), Some(37.0));
        assert_eq!(number_from_value(pick_path(&v, "items.0.count").unwrap()), Some(37.0));
        assert_eq!(number_from_value(pick_path(&v, r#"["a.b"]"#).unwrap()), Some(5.0));
        assert!(pick_path(&v, "items[5].count").is_none());
        assert!(pick_path(&v, "items[x]").is_none());
        let root = serde_json::json!(37);
        assert_eq!(number_from_value(pick_path(&root, "").unwrap()), Some(37.0));
        let arr = serde_json::json!([{"n": 1}, {"n": 2}]);
        assert_eq!(number_from_value(pick_path(&arr, "[1].n").unwrap()), Some(2.0));
    }

    #[test]
    fn json_path_on_html_is_error() {
        let e = extract("<html>Login 2024</html>", &Pick::JsonPath("a".into())).unwrap_err();
        assert_eq!(e, "response is not JSON");
    }

    #[test]
    fn missing_path_message() {
        let e = extract(r#"{"a":1}"#, &Pick::JsonPath("data.subs".into())).unwrap_err();
        assert_eq!(e, "`data.subs` not found in response");
        let e = extract(r#"{"a":"x"}"#, &Pick::JsonPath("a".into())).unwrap_err();
        assert_eq!(e, "`a` is not a number");
        let e = extract(r#"{"a":true}"#, &Pick::JsonPath("a".into())).unwrap_err();
        assert_eq!(e, "`a` is not a number");
    }

    #[test]
    fn first_number_in_text() {
        assert_eq!(first_number("Orders today: 37 (+6)"), Some(37.0));
        assert_eq!(first_number("1,284 subs"), Some(1284.0));
        assert_eq!(first_number("balance -12.5 EUR"), Some(-12.5));
        assert_eq!(first_number("views 2.5k today"), Some(2500.0));
        assert_eq!(first_number("version 3 kinds"), Some(3.0));
        assert_eq!(first_number("end."), None);
        assert_eq!(first_number("12."), Some(12.0));
    }

    #[test]
    fn regex_pick() {
        assert_eq!(extract("subs=48,213;", &Pick::Regex(r"subs=([\d,]+)".into())).unwrap(), 48213.0);
        assert!(extract("x", &Pick::Regex("(".into())).unwrap_err().starts_with("invalid pattern"));
        assert_eq!(
            extract("nothing", &Pick::Regex(r"subs=(\d+)".into())).unwrap_err(),
            "pattern did not match the response"
        );
    }
}
