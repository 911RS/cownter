use super::{FetchError, Fetcher};
use crate::model::LemonSqueezyMetric;

/// Store totals, summed over all stores the key can see. Revenue is in the store currency.
pub(super) async fn fetch(f: &Fetcher, metric: LemonSqueezyMetric, key_name: &str) -> Result<f64, FetchError> {
    let key = f.secret(key_name)?;
    let base = f.base(f.base.lemonsqueezy.as_ref(), "https://api.lemonsqueezy.com");
    let req = f.client.get(format!("{base}/v1/stores")).header("Accept", "application/vnd.api+json").bearer_auth(key);
    let v = f.json(req).await?;
    let stores = v.get("data").and_then(|d| d.as_array()).ok_or_else(|| FetchError::Parse("`data` missing in response".into()))?;
    let field = match metric {
        LemonSqueezyMetric::Sales => "total_sales",
        LemonSqueezyMetric::Revenue => "total_revenue",
        LemonSqueezyMetric::Revenue30d => "thirty_day_revenue",
    };
    let total: f64 = stores
        .iter()
        .filter_map(|s| s.get("attributes").and_then(|a| a.get(field)).and_then(|n| n.as_f64()))
        .sum();
    Ok(if metric == LemonSqueezyMetric::Sales { total } else { total / 100.0 })
}
