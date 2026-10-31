use super::{FetchError, Fetcher, local_midnight};
use crate::parse::number_from_value;
use chrono::{DateTime, Local};

pub const API_VERSION: &str = "2025-10";

fn valid_shop(s: &str) -> bool {
    !s.is_empty()
        && s.contains('.')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'))
        && !s.starts_with('.')
        && !s.ends_with('.')
}

pub(super) async fn fetch(f: &Fetcher, shop: &str, token_name: &str, now: DateTime<Local>) -> Result<f64, FetchError> {
    let shop = shop.trim().trim_start_matches("https://").trim_end_matches('/').to_ascii_lowercase();
    if !valid_shop(&shop) {
        return Err(FetchError::Config("enter the shop domain, e.g. my-store.myshopify.com".into()));
    }
    let token = f.secret(token_name)?;
    let base = f.base(f.base.shopify.as_ref(), &format!("https://{shop}"));
    let since = local_midnight(now).to_rfc3339();
    let url = format!("{base}/admin/api/{API_VERSION}/orders/count.json");
    let req = f
        .client
        .get(url)
        .query(&[("status", "any"), ("created_at_min", since.as_str())])
        .header("X-Shopify-Access-Token", token);
    let v = f.json(req).await?;
    v.get("count").and_then(number_from_value).ok_or_else(|| FetchError::Parse("`count` missing in response".into()))
}
