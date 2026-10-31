use super::{FetchError, Fetcher};
use crate::model::YouTubeMetric;
use crate::parse::number_from_value;

pub(super) async fn fetch(f: &Fetcher, channel_id: &str, metric: YouTubeMetric, key_name: &str) -> Result<f64, FetchError> {
    let channel_id = channel_id.trim();
    if channel_id.is_empty() {
        return Err(FetchError::Config("channel ID is required".into()));
    }
    let key = f.secret(key_name)?;
    let base = f.base(f.base.youtube.as_ref(), "https://www.googleapis.com");
    let url = format!("{base}/youtube/v3/channels");
    let v = f
        .json(f.client.get(url).query(&[("part", "statistics"), ("id", channel_id), ("key", key.as_str())]))
        .await?;
    let stats = v
        .get("items")
        .and_then(|i| i.get(0))
        .and_then(|i| i.get("statistics"))
        .ok_or_else(|| FetchError::Parse("channel not found; check the channel ID".into()))?;
    let field = match metric {
        YouTubeMetric::Subscribers => "subscriberCount",
        YouTubeMetric::Views => "viewCount",
        YouTubeMetric::Videos => "videoCount",
    };
    stats
        .get(field)
        .and_then(number_from_value)
        .ok_or_else(|| FetchError::Parse(format!("`{field}` is hidden or missing for this channel")))
}
