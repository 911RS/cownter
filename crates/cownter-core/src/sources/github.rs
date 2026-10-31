use super::{FetchError, Fetcher};
use crate::model::GitHubMetric;
use crate::parse::number_from_value;

fn valid_part(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub(super) async fn fetch(f: &Fetcher, target: &str, metric: GitHubMetric, token: Option<&str>) -> Result<f64, FetchError> {
    let target = target.trim().trim_start_matches("https://github.com/").trim_matches('/');
    let parts: Vec<&str> = target.split('/').collect();
    let is_repo = parts.len() == 2;
    if !(parts.len() == 1 || is_repo) || !parts.iter().all(|p| valid_part(p)) {
        return Err(FetchError::Config("enter `owner/repo` or a user name".into()));
    }
    if metric.is_repo_metric() != is_repo {
        return Err(FetchError::Config(if is_repo {
            "this metric needs a user name, not a repository".into()
        } else {
            "this metric needs `owner/repo`".into()
        }));
    }
    let base = f.base(f.base.github.as_ref(), "https://api.github.com");
    let url = if is_repo { format!("{base}/repos/{target}") } else { format!("{base}/users/{target}") };
    let mut req = f
        .client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(name) = token {
        req = req.bearer_auth(f.secret(name)?);
    }
    let v = match f.json(req).await {
        Err(FetchError::Http(404)) => return Err(FetchError::Parse(format!("`{target}` was not found on GitHub"))),
        other => other?,
    };
    let field = match metric {
        GitHubMetric::Stars => "stargazers_count",
        GitHubMetric::Forks => "forks_count",
        GitHubMetric::OpenIssues => "open_issues_count",
        GitHubMetric::Watchers => "subscribers_count",
        GitHubMetric::Followers => "followers",
        GitHubMetric::PublicRepos => "public_repos",
    };
    v.get(field).and_then(number_from_value).ok_or_else(|| FetchError::Parse(format!("`{field}` missing in response")))
}
