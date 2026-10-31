use super::{FetchError, Fetcher};
use crate::parse::number_from_value;

const QUERY: &str = "query($slug: String) { post(slug: $slug) { votesCount } }";

fn slug_of(post: &str) -> &str {
    let p = post.trim();
    let p = p.split(['?', '#']).next().unwrap_or(p).trim_end_matches('/');
    p.rsplit_once("/posts/").map(|(_, s)| s).unwrap_or(p)
}

pub(super) async fn fetch(f: &Fetcher, post: &str, token_name: &str) -> Result<f64, FetchError> {
    let slug = slug_of(post);
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(FetchError::Config("enter the post slug or its producthunt.com/posts/… link".into()));
    }
    let token = f.secret(token_name)?;
    let base = f.base(f.base.producthunt.as_ref(), "https://api.producthunt.com");
    let body = serde_json::json!({ "query": QUERY, "variables": { "slug": slug } });
    let v = f.json(f.client.post(format!("{base}/v2/api/graphql")).bearer_auth(token).json(&body)).await?;
    if let Some(msg) = v.pointer("/errors/0/message").and_then(|m| m.as_str()) {
        return Err(FetchError::Parse(format!("Product Hunt: {msg}")));
    }
    let post = v.pointer("/data/post").filter(|p| !p.is_null());
    let post = post.ok_or_else(|| FetchError::Parse(format!("post `{slug}` was not found on Product Hunt")))?;
    post.get("votesCount").and_then(number_from_value).ok_or_else(|| FetchError::Parse("`votesCount` missing in response".into()))
}

#[cfg(test)]
mod tests {
    use super::slug_of;

    #[test]
    fn slugs_from_links() {
        assert_eq!(slug_of("cownter"), "cownter");
        assert_eq!(slug_of("https://www.producthunt.com/posts/cownter-2/"), "cownter-2");
        assert_eq!(slug_of("https://www.producthunt.com/posts/cownter?ref=x"), "cownter");
    }
}
