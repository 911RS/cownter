use super::{FetchError, Fetcher, local_midnight};
use chrono::{DateTime, Local};

const ZERO_DECIMAL: &[&str] = &[
    "bif", "clp", "djf", "gnf", "jpy", "kmf", "krw", "mga", "pyg", "rwf", "ugx", "vnd", "vuv", "xaf", "xof", "xpf",
];
const MAX_PAGES: usize = 50;

/// Today's revenue: succeeded charges in `currency`, minus refunded amounts, in major units.
pub(super) async fn fetch(f: &Fetcher, key_name: &str, currency: &str, now: DateTime<Local>) -> Result<f64, FetchError> {
    let currency = currency.trim().to_ascii_lowercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(FetchError::Config("currency must be a 3-letter code like EUR".into()));
    }
    let key = f.secret(key_name)?;
    let base = f.base(f.base.stripe.as_ref(), "https://api.stripe.com");
    let since = local_midnight(now).timestamp().to_string();
    let mut total: i64 = 0;
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut q = vec![("created[gte]", since.clone()), ("limit", "100".to_string())];
        if let Some(a) = &after {
            q.push(("starting_after", a.clone()));
        }
        let v = f.json(f.client.get(format!("{base}/v1/charges")).bearer_auth(&key).query(&q)).await?;
        let data = v
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| FetchError::Parse("`data` missing in response".into()))?;
        for ch in data {
            let ok = ch.get("status").and_then(|s| s.as_str()) == Some("succeeded")
                && ch.get("currency").and_then(|s| s.as_str()).map(|s| s.to_ascii_lowercase()) == Some(currency.clone());
            if ok {
                let amount = ch.get("amount").and_then(|a| a.as_i64()).unwrap_or(0);
                let refunded = ch.get("amount_refunded").and_then(|a| a.as_i64()).unwrap_or(0);
                total += amount - refunded;
            }
        }
        let more = v.get("has_more").and_then(|h| h.as_bool()).unwrap_or(false);
        after = data.last().and_then(|c| c.get("id")).and_then(|i| i.as_str()).map(str::to_string);
        if !more || after.is_none() {
            break;
        }
    }
    let divisor = if ZERO_DECIMAL.contains(&currency.as_str()) { 1.0 } else { 100.0 };
    Ok(total as f64 / divisor)
}
