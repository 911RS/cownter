use super::{FetchError, Fetcher};
use crate::model::PlausibleMetric;
use crate::parse::number_from_value;

pub(super) async fn fetch(f: &Fetcher, site_id: &str, metric: PlausibleMetric, key_name: &str) -> Result<f64, FetchError> {
    let site = site_id.trim().trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
    if site.is_empty() || site.contains(char::is_whitespace) {
        return Err(FetchError::Config("enter the site domain as added in Plausible, e.g. example.com".into()));
    }
    let key = f.secret(key_name)?;
    let base = f.base(f.base.plausible.as_ref(), "https://plausible.io");
    match metric {
        PlausibleMetric::Realtime => {
            let req = f.client.get(format!("{base}/api/v1/stats/realtime/visitors")).query(&[("site_id", site)]).bearer_auth(key);
            let body = f.send(req).await?;
            body.text.trim().parse::<f64>().map_err(|_| FetchError::Parse("expected a number of visitors".into()))
        }
        PlausibleMetric::VisitorsToday => {
            let req = f
                .client
                .get(format!("{base}/api/v1/stats/aggregate"))
                .query(&[("site_id", site), ("period", "day"), ("metrics", "visitors")])
                .bearer_auth(key);
            let v = f.json(req).await?;
            v.pointer("/results/visitors/value")
                .and_then(number_from_value)
                .ok_or_else(|| FetchError::Parse("`results.visitors.value` missing in response".into()))
        }
    }
}
