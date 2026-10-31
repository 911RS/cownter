use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Live,
    Polling,
    Stale,
    Error,
    NoLink,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Live => "live",
            Status::Polling => "polling",
            Status::Stale => "stale",
            Status::Error => "error",
            Status::NoLink => "noLink",
        }
    }

    pub fn parse(s: &str) -> Status {
        match s {
            "live" => Status::Live,
            "polling" => Status::Polling,
            "stale" => Status::Stale,
            "noLink" => Status::NoLink,
            _ => Status::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Pick {
    JsonPath(String),
    FirstNumber,
    Regex(String),
}

/// A header value is either plain text or the name of a keyring secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum SecretOrText {
    Text(String),
    Secret(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: SecretOrText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum YouTubeMetric {
    Subscribers,
    Views,
    Videos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitHubMetric {
    Stars,
    Forks,
    OpenIssues,
    Watchers,
    Followers,
    PublicRepos,
}

impl GitHubMetric {
    pub fn is_repo_metric(self) -> bool {
        matches!(
            self,
            GitHubMetric::Stars | GitHubMetric::Forks | GitHubMetric::OpenIssues | GitHubMetric::Watchers
        )
    }
}

/// Secret-bearing fields (`api_key`, `token`) hold keyring names, never secret values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SourceCfg {
    CustomUrl {
        url: String,
        method: String,
        headers: Vec<Header>,
        body: Option<String>,
        pick: Pick,
    },
    #[serde(rename = "youtube")]
    YouTube {
        channel_id: String,
        metric: YouTubeMetric,
        api_key: String,
    },
    #[serde(rename = "github")]
    GitHub {
        target: String,
        metric: GitHubMetric,
        token: Option<String>,
    },
    Shopify {
        shop: String,
        token: String,
    },
    Stripe {
        api_key: String,
        currency: String,
    },
    Formula {
        expr: String,
    },
}

impl SourceCfg {
    pub fn is_formula(&self) -> bool {
        matches!(self, SourceCfg::Formula { .. })
    }

    /// Keyring names this source refers to.
    pub fn secret_names(&self) -> Vec<String> {
        match self {
            SourceCfg::CustomUrl { headers, .. } => headers
                .iter()
                .filter_map(|h| match &h.value {
                    SecretOrText::Secret(n) => Some(n.clone()),
                    SecretOrText::Text(_) => None,
                })
                .collect(),
            SourceCfg::YouTube { api_key, .. } => vec![api_key.clone()],
            SourceCfg::GitHub { token, .. } => token.iter().cloned().collect(),
            SourceCfg::Shopify { token, .. } => vec![token.clone()],
            SourceCfg::Stripe { api_key, .. } => vec![api_key.clone()],
            SourceCfg::Formula { .. } => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    /// `emoji`, `builtin`, `image` (data URL) or `favicon` (data URL).
    pub kind: String,
    pub value: String,
    pub bg: String,
}

impl Default for Icon {
    fn default() -> Self {
        Icon { kind: "emoji".into(), value: "📈".into(), bg: "#2b2f3a".into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum NumberStyle {
    #[default]
    Full,
    Compact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NumberFormat {
    pub style: NumberStyle,
    pub decimals: Option<u8>,
    pub prefix: String,
    pub suffix: String,
}

/// Sound names refer to the built-in sound set in the UI (`chime`, `bubble`, `tick`, `bell`, `pop`, `marimba`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Sounds {
    pub on_change: Option<String>,
    pub on_goal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub icon: Icon,
    pub group_id: Option<i64>,
    pub source: SourceCfg,
    pub interval_s: u32,
    pub format: NumberFormat,
    pub goal_value: Option<f64>,
    pub celebrate: bool,
    #[serde(default)]
    pub sounds: Sounds,
    pub sort: i64,
    /// Shows the fetched text instead of a number.
    #[serde(default)]
    pub text_mode: bool,
    /// Text entries: the goal is reached when the text becomes this value.
    #[serde(default)]
    pub goal_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AlertKind {
    Above,
    Below,
    ChangePct,
    RiseBy,
    DropBy,
}

impl AlertKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AlertKind::Above => "above",
            AlertKind::Below => "below",
            AlertKind::ChangePct => "changePct",
            AlertKind::RiseBy => "riseBy",
            AlertKind::DropBy => "dropBy",
        }
    }

    pub fn parse(s: &str) -> Option<AlertKind> {
        match s {
            "above" => Some(AlertKind::Above),
            "below" => Some(AlertKind::Below),
            "changePct" => Some(AlertKind::ChangePct),
            "riseBy" => Some(AlertKind::RiseBy),
            "dropBy" => Some(AlertKind::DropBy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: i64,
    pub entry_id: i64,
    pub kind: AlertKind,
    pub threshold: f64,
    pub window_s: u32,
    pub enabled: bool,
    pub last_fired_at: Option<i64>,
    /// Show a desktop notification when it fires.
    #[serde(default = "yes")]
    pub notify: bool,
    /// Effect shown on the widgets when it fires: empty, `float`, `pop` or `burst`.
    #[serde(default)]
    pub effect: String,
}

fn yes() -> bool {
    true
}

pub const ALERT_EFFECTS: [&str; 10] = ["", "float", "pop", "burst", "confetti", "shake", "glow", "flash", "zoom", "rain"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub sort: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub entry_id: i64,
    pub ts: i64,
    pub value: Option<f64>,
    pub status: Status,
    pub error: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryView {
    pub entry: Entry,
    pub value: Option<f64>,
    pub prev: Option<f64>,
    pub status: Status,
    pub error: Option<String>,
    pub updated_at: Option<i64>,
    pub delta_today: Option<f64>,
    pub spark: Vec<(i64, f64)>,
    pub goal_progress: Option<f64>,
    pub text: Option<String>,
    pub prev_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CoreEvent {
    EntryUpdated { view: EntryView },
    EntryRemoved { entry_id: i64 },
    ValueChanged { entry_id: i64, value: f64, prev: f64 },
    AlertFired { entry_id: i64, alert_id: i64, message: String, value: f64, diff: Option<f64>, notify: bool, effect: String },
    TextChanged { entry_id: i64, text: String, prev: String },
    GoalReached { entry_id: i64, value: f64, text: Option<String> },
    AllTimeHigh { entry_id: i64, value: f64 },
    AuthFailed { entry_id: i64, message: String },
}

pub fn valid_slug(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && s.len() <= 40
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_json_shape() {
        let s = SourceCfg::YouTube {
            channel_id: "UC1".into(),
            metric: YouTubeMetric::Subscribers,
            api_key: "entry-1-apiKey".into(),
        };
        let j = serde_json::to_value(&s).unwrap();
        assert_eq!(j["type"], "youtube");
        assert_eq!(j["channelId"], "UC1");
        assert_eq!(j["apiKey"], "entry-1-apiKey");
        let p = serde_json::to_value(Pick::FirstNumber).unwrap();
        assert_eq!(p, serde_json::json!({"kind": "firstNumber"}));
    }

    #[test]
    fn slugs() {
        assert!(valid_slug("orders"));
        assert!(valid_slug("yt_subs2"));
        assert!(!valid_slug("2subs"));
        assert!(!valid_slug("Orders"));
        assert!(!valid_slug(""));
        assert!(!valid_slug("a-b"));
    }
}
