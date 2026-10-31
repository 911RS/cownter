use super::{FetchError, Fetcher};
use crate::model::GumroadMetric;

/// Sales count or revenue in USD, summed over all products.
pub(super) async fn fetch(f: &Fetcher, metric: GumroadMetric, token_name: &str) -> Result<f64, FetchError> {
    let token = f.secret(token_name)?;
    let base = f.base(f.base.gumroad.as_ref(), "https://api.gumroad.com");
    let v = f.json(f.client.get(format!("{base}/v2/products")).query(&[("access_token", token.as_str())])).await?;
    let products = v
        .get("products")
        .and_then(|p| p.as_array())
        .ok_or_else(|| FetchError::Parse("`products` missing in response".into()))?;
    let field = match metric {
        GumroadMetric::Sales => "sales_count",
        GumroadMetric::Revenue => "sales_usd_cents",
    };
    let total: f64 = products.iter().filter_map(|p| p.get(field).and_then(|n| n.as_f64())).sum();
    Ok(if metric == GumroadMetric::Revenue { total / 100.0 } else { total })
}
