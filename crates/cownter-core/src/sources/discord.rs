use super::{FetchError, Fetcher};
use crate::model::DiscordMetric;
use crate::parse::number_from_value;

fn code_of(invite: &str) -> &str {
    let s = invite.trim();
    let s = s.split(['?', '#']).next().unwrap_or(s).trim_end_matches('/');
    s.rsplit('/').next().unwrap_or(s)
}

pub(super) async fn fetch(f: &Fetcher, invite: &str, metric: DiscordMetric) -> Result<f64, FetchError> {
    let code = code_of(invite);
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(FetchError::Config("enter an invite code or a discord.gg/… link".into()));
    }
    let base = f.base(f.base.discord.as_ref(), "https://discord.com");
    let req = f.client.get(format!("{base}/api/v10/invites/{code}")).query(&[("with_counts", "true")]);
    let v = match f.json(req).await {
        Err(FetchError::Http(404)) => return Err(FetchError::Parse("this invite is invalid or has expired".into())),
        other => other?,
    };
    let field = match metric {
        DiscordMetric::Members => "approximate_member_count",
        DiscordMetric::Online => "approximate_presence_count",
    };
    v.get(field).and_then(number_from_value).ok_or_else(|| FetchError::Parse(format!("`{field}` missing in response")))
}

#[cfg(test)]
mod tests {
    use super::code_of;

    #[test]
    fn codes_from_links() {
        assert_eq!(code_of("abc123"), "abc123");
        assert_eq!(code_of("https://discord.gg/abc123"), "abc123");
        assert_eq!(code_of("https://discord.com/invite/abc123/?x=1"), "abc123");
    }
}
