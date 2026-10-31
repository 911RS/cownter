use super::{FetchError, Fetcher};
use crate::parse::number_from_value;

fn valid_package(s: &str) -> bool {
    let name = match s.strip_prefix('@') {
        Some(rest) => match rest.split_once('/') {
            Some((scope, name)) if !scope.is_empty() => {
                if !scope.chars().all(valid_char) {
                    return false;
                }
                name
            }
            _ => return false,
        },
        None => s,
    };
    !name.is_empty() && name.len() <= 214 && name.chars().all(valid_char)
}

fn valid_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.' | '~')
}

/// Downloads over the last 7 days.
pub(super) async fn fetch(f: &Fetcher, package: &str) -> Result<f64, FetchError> {
    let package = package.trim().trim_start_matches("https://www.npmjs.com/package/").trim_matches('/');
    if !valid_package(package) {
        return Err(FetchError::Config("enter a package name like react or @scope/name".into()));
    }
    let base = f.base(f.base.npm.as_ref(), "https://api.npmjs.org");
    let v = match f.json(f.client.get(format!("{base}/downloads/point/last-week/{package}"))).await {
        Err(FetchError::Http(404)) => return Err(FetchError::Parse(format!("`{package}` was not found on npm"))),
        other => other?,
    };
    v.get("downloads").and_then(number_from_value).ok_or_else(|| FetchError::Parse("`downloads` missing in response".into()))
}
