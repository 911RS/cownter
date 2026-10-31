use cownter_core::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "display";
const VERSION: u32 = 4;

/// Forms beyond island, edge and pop, with their default anchor.
pub const EXTRA_FORMS: &[(&str, &str)] = &[
    ("ticker", "bottomCenter"),
    ("orb", "bottomRight"),
    ("chips", "topRight"),
    ("stream", "bottomLeft"),
    ("peek", "middleCenter"),
    ("trend", "topRight"),
    ("tag", "topRight"),
    ("deck", "bottomCenter"),
    ("notch", "topCenter"),
    ("folders", "topLeft"),
    ("clothesline", "topCenter"),
    ("train", "bottomLeft"),
    ("drawer", "middleLeft"),
    ("feed", "bottomRight"),
    ("pet", "bottomRight"),
    ("pager", "bottomRight"),
    ("wings", "topCenter"),
    ("pills", "bottomCenter"),
    ("bar", "topCenter"),
    ("versus", "topCenter"),
    ("toptabs", "topCenter"),
];

const POSITIONS: &[&str] = &["topLeft", "topCenter", "topRight", "middleLeft", "middleCenter", "middleRight", "bottomLeft", "bottomCenter", "bottomRight"];

fn extra_default(id: &str, position: &str) -> FormCfg {
    let side = if id == "drawer" { "left" } else { "right" };
    FormCfg { position: position.into(), side: side.into(), offset: 0.5, ..FormCfg::default() }
}

fn default_extras() -> BTreeMap<String, FormCfg> {
    EXTRA_FORMS.iter().map(|(id, pos)| (id.to_string(), extra_default(id, pos))).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FormCfg {
    pub enabled: bool,
    /// A font look (`clean`, `serif`, `poster`, ...) or a retro look (`dotMatrix`, `calculator`, `neon`).
    pub skin: String,
    /// `white`, `chroma`, `amber`, `phosphor`, `ice`, `ruby`, `violet` or `classic`.
    pub tint: String,
    /// `xs`, `s`, `m`, `l` or `xl`.
    pub size: String,
    pub group_id: Option<i64>,
    /// Explicit entries to show; empty means every entry (of the group, if set).
    pub entry_ids: Vec<i64>,
    /// Edge tab: `right` or `left`.
    pub side: String,
    /// Edge tab: vertical position, 0 = top, 1 = bottom.
    pub offset: f64,
    /// Island and pop: `topLeft`, `topCenter`, `topRight`, `bottomLeft`, `bottomCenter` or `bottomRight`.
    pub position: String,
    /// Island: seconds per entry while cycling.
    pub cycle_s: u32,
    /// Nudge from the anchored position, in logical pixels.
    pub offset_x: f64,
    pub offset_y: f64,
    /// Monitor name; empty means the primary monitor.
    pub monitor: String,
    /// Extra scale on top of `size`, 0.6 to 2.
    pub scale: f64,
    /// Whole-widget opacity, 0.2 to 1.
    pub opacity: f64,
    /// Background colour as `#rrggbb`.
    pub bg: String,
    /// Background opacity, 0 to 1.
    pub bg_alpha: f64,
    /// Corner roundness, 0 (square) to 1 (pill).
    pub roundness: f64,
    /// Space between items, 0 to 3 times the normal spacing.
    pub spacing: f64,
    /// Notch wings: gap between the two wings, in logical pixels.
    pub gap: f64,
    pub shadow: bool,
    pub show_icons: bool,
    /// `tile`, `big`, `fade`, `glow`, `glyph` or `bar`.
    pub icon_style: String,
    pub show_delta: bool,
    /// Island: the page dots next to the number.
    pub show_dots: bool,
    /// Show entry names where the form has room for them.
    pub show_names: bool,
    /// Click opens settings when true; otherwise clicks do nothing.
    pub click_opens: bool,
    /// Hover expands the widget when true.
    pub hover_expands: bool,
    /// Per-look switches, keyed `skin.option`.
    pub skin_opts: BTreeMap<String, bool>,
    /// Per-form switches such as indicators, keyed by option name.
    pub form_opts: BTreeMap<String, bool>,
}

impl Default for FormCfg {
    fn default() -> Self {
        FormCfg {
            enabled: false,
            skin: "clean".into(),
            tint: "white".into(),
            size: "m".into(),
            group_id: None,
            entry_ids: vec![],
            side: "right".into(),
            offset: 0.35,
            position: "topRight".into(),
            cycle_s: 4,
            offset_x: 0.0,
            offset_y: 0.0,
            monitor: String::new(),
            scale: 1.0,
            opacity: 1.0,
            bg: "#000000".into(),
            bg_alpha: 1.0,
            roundness: 1.0,
            spacing: 1.0,
            gap: 26.0,
            shadow: true,
            show_icons: true,
            icon_style: "tile".into(),
            show_delta: true,
            show_dots: true,
            show_names: true,
            click_opens: true,
            hover_expands: true,
            skin_opts: BTreeMap::new(),
            form_opts: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrayCfg {
    pub show_number: bool,
    pub entry_id: Option<i64>,
}

impl Default for TrayCfg {
    fn default() -> Self {
        TrayCfg { show_number: true, entry_id: None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayConfig {
    /// Config format version; older saves are migrated in `sanitized`.
    #[serde(default)]
    pub version: u32,
    pub island: FormCfg,
    pub edge: FormCfg,
    pub pop: FormCfg,
    /// Every other form, keyed by its id from `EXTRA_FORMS`.
    pub forms: BTreeMap<String, FormCfg>,
    pub celebration: bool,
    pub tray: TrayCfg,
    pub volume: f64,
    pub muted: bool,
    pub hotkey_toggle: String,
    pub hotkey_refresh: String,
    pub hotkey_peek: String,
    pub launch_at_login: bool,
    pub hidden: bool,
    pub onboarded: bool,
    pub weekly_recap: bool,
    /// Day the weekly recap shows, 0 = Monday to 6 = Sunday.
    pub recap_weekday: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            version: VERSION,
            island: FormCfg { enabled: true, position: "topCenter".into(), offset_y: 8.0, ..FormCfg::default() },
            edge: FormCfg { skin: "tall".into(), ..FormCfg::default() },
            pop: FormCfg { enabled: true, ..FormCfg::default() },
            forms: default_extras(),
            celebration: true,
            tray: TrayCfg::default(),
            volume: 0.6,
            muted: false,
            hotkey_toggle: "CommandOrControl+Shift+U".into(),
            hotkey_refresh: "CommandOrControl+Shift+R".into(),
            hotkey_peek: "CommandOrControl+Shift+P".into(),
            launch_at_login: false,
            hidden: false,
            onboarded: false,
            weekly_recap: true,
            recap_weekday: 0,
        }
    }
}

impl DisplayConfig {
    pub fn load(store: &Store) -> DisplayConfig {
        store
            .setting(KEY)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<DisplayConfig>(&s).ok())
            .unwrap_or_default()
            .sanitized()
    }

    pub fn save(&self, store: &Store) -> Result<(), String> {
        let s = serde_json::to_string(self).map_err(|e| e.to_string())?;
        store.set_setting(KEY, &s).map_err(|e| e.to_string())
    }

    /// Config of any form by its window label.
    pub fn form(&self, id: &str) -> Option<&FormCfg> {
        match id {
            "island" => Some(&self.island),
            "edge" => Some(&self.edge),
            "pop" => Some(&self.pop),
            other => self.forms.get(other),
        }
    }

    pub fn sanitized(mut self) -> DisplayConfig {
        if self.version < 2 {
            self.island.position = "topCenter".into();
            self.island.offset_y = 8.0;
        }
        if self.version < 3 {
            for f in [&mut self.island, &mut self.edge, &mut self.pop].into_iter().chain(self.forms.values_mut()) {
                if matches!(f.skin.as_str(), "dotMatrix" | "splitFlap" | "nixie" | "odometer") && f.tint == "amber" {
                    f.skin = "clean".into();
                    f.tint = "white".into();
                }
            }
        }
        if self.version < 4 {
            for f in [&mut self.island, &mut self.edge, &mut self.pop].into_iter().chain(self.forms.values_mut()) {
                let opt = match f.skin.as_str() {
                    "heavy" => Some("bold"),
                    "light" => Some("thin"),
                    "gradient" => Some("gradient"),
                    "outline" => Some("outline"),
                    _ => None,
                };
                if let Some(o) = opt {
                    let old = f.skin.clone();
                    f.skin = "clean".into();
                    let carried: Vec<(String, bool)> = f
                        .skin_opts
                        .iter()
                        .filter_map(|(k, v)| k.strip_prefix(&format!("{old}.")).map(|r| (format!("clean.{r}"), *v)))
                        .collect();
                    f.skin_opts.extend(carried);
                    f.skin_opts.insert(format!("clean.{o}"), true);
                }
            }
        }
        self.version = VERSION;
        self.forms.retain(|k, _| EXTRA_FORMS.iter().any(|(id, _)| id == k));
        for (id, pos) in EXTRA_FORMS {
            self.forms.entry(id.to_string()).or_insert_with(|| extra_default(id, pos));
        }
        for f in [&mut self.island, &mut self.edge, &mut self.pop].into_iter().chain(self.forms.values_mut()) {
            if !matches!(f.skin.as_str(), "clean" | "dmsans" | "manrope" | "poppins" | "montserrat" | "rubik" | "lexend" | "geometric" | "modern" | "rounded" | "grotesk" | "tall" | "barlow" | "poster" | "wide" | "future" | "mono" | "plexmono" | "spacemono" | "dotMatrix" | "calculator" | "neon") {
                f.skin = "clean".into();
            }
            if !matches!(f.tint.as_str(), "white" | "chroma" | "amber" | "phosphor" | "ice" | "ruby" | "violet" | "classic") {
                f.tint = "white".into();
            }
            if !matches!(f.icon_style.as_str(), "tile" | "big" | "fade" | "glow" | "glyph" | "bar") {
                f.icon_style = "tile".into();
            }
            if !matches!(f.size.as_str(), "xs" | "s" | "m" | "l" | "xl") {
                f.size = "m".into();
            }
            if !matches!(f.side.as_str(), "left" | "right") {
                f.side = "right".into();
            }
            if !POSITIONS.contains(&f.position.as_str()) {
                f.position = "topRight".into();
            }
            let fin = |v: f64, lo: f64, hi: f64, d: f64| if v.is_finite() { v.clamp(lo, hi) } else { d };
            f.offset_x = fin(f.offset_x, -4000.0, 4000.0, 0.0);
            f.offset_y = fin(f.offset_y, -4000.0, 4000.0, 0.0);
            f.scale = fin(f.scale, 0.6, 2.0, 1.0);
            f.opacity = fin(f.opacity, 0.2, 1.0, 1.0);
            f.bg_alpha = fin(f.bg_alpha, 0.0, 1.0, 1.0);
            f.roundness = fin(f.roundness, 0.0, 1.0, 1.0);
            f.spacing = fin(f.spacing, 0.0, 3.0, 1.0);
            f.gap = fin(f.gap, 0.0, 800.0, 26.0);
            let hex = f.bg.len() == 7 && f.bg.starts_with('#') && f.bg[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !hex {
                f.bg = "#000000".into();
            }
            f.offset = if f.offset.is_finite() { f.offset.clamp(0.0, 1.0) } else { 0.35 };
            f.cycle_s = f.cycle_s.clamp(2, 60);
            let key_ok = |k: &String| k.len() <= 40 && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
            f.skin_opts.retain(|k, _| key_ok(k));
            f.form_opts.retain(|k, _| key_ok(k));
            while f.skin_opts.len() > 128 {
                f.skin_opts.pop_last();
            }
            while f.form_opts.len() > 64 {
                f.form_opts.pop_last();
            }
        }
        self.recap_weekday = self.recap_weekday.min(6);
        self.volume = if self.volume.is_finite() { self.volume.clamp(0.0, 1.0) } else { 0.6 };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let s = Store::open_memory().unwrap();
        assert_eq!(DisplayConfig::load(&s), DisplayConfig::default());
        let mut c = DisplayConfig::default();
        c.edge.enabled = true;
        c.save(&s).unwrap();
        assert!(DisplayConfig::load(&s).edge.enabled);
    }

    #[test]
    fn partial_json_fills_defaults() {
        let c: DisplayConfig = serde_json::from_str(r#"{"island":{"tint":"ice"}}"#).unwrap();
        assert_eq!(c.island.tint, "ice");
        assert_eq!(c.island.skin, "clean");
        assert_eq!(c.hotkey_toggle, "CommandOrControl+Shift+U");
        assert!(c.weekly_recap);
        assert_eq!(c.recap_weekday, 0);
    }

    #[test]
    fn sanitizes_bad_values() {
        let mut c = DisplayConfig::default();
        c.island.skin = "x".into();
        c.edge.offset = 9.0;
        c.volume = f64::NAN;
        c.recap_weekday = 12;
        let c = c.sanitized();
        assert_eq!(c.recap_weekday, 6);
        assert_eq!(c.island.skin, "clean");
        assert_eq!(c.edge.offset, 1.0);
        assert_eq!(c.volume, 0.6);
    }

    #[test]
    fn old_saves_keep_island_centred() {
        let c: DisplayConfig = serde_json::from_str(r#"{"island":{"enabled":true,"position":"topRight"}}"#).unwrap();
        let c = c.sanitized();
        assert_eq!(c.island.position, "topCenter");
        assert_eq!(c.version, VERSION);
        let again: DisplayConfig = serde_json::from_str(&serde_json::to_string(&DisplayConfig { island: FormCfg { position: "bottomLeft".into(), ..c.island.clone() }, ..c }).unwrap()).unwrap();
        assert_eq!(again.sanitized().island.position, "bottomLeft");
    }

    #[test]
    fn extra_forms_are_filled_and_unknown_dropped() {
        let c: DisplayConfig = serde_json::from_str(r#"{"forms":{"ticker":{"enabled":true},"bogus":{}}}"#).unwrap();
        let c = c.sanitized();
        assert_eq!(c.forms.len(), EXTRA_FORMS.len());
        assert!(c.forms["ticker"].enabled);
        assert!(!c.forms.contains_key("bogus"));
        assert_eq!(c.form("drawer").unwrap().side, "left");
        assert_eq!(c.form("peek").unwrap().position, "middleCenter");
    }

    #[test]
    fn sanitizes_look_and_offsets() {
        let mut c = DisplayConfig::default();
        c.pop.bg = "red; x".into();
        c.pop.scale = 50.0;
        c.pop.offset_x = f64::INFINITY;
        c.pop.opacity = 0.0;
        c.pop.position = "middle".into();
        let c = c.sanitized();
        assert_eq!(c.pop.bg, "#000000");
        assert_eq!(c.pop.scale, 2.0);
        assert_eq!(c.pop.offset_x, 0.0);
        assert_eq!(c.pop.opacity, 0.2);
        assert_eq!(c.pop.position, "topRight");
    }

    #[test]
    fn keeps_valid_options_only() {
        let mut c = DisplayConfig::default();
        c.island.skin_opts.insert("dotMatrix.panel".into(), true);
        c.island.skin_opts.insert("bad key;".into(), true);
        c.island.form_opts.insert("dots".into(), false);
        let c = c.sanitized();
        assert_eq!(c.island.skin_opts.get("dotMatrix.panel"), Some(&true));
        assert_eq!(c.island.skin_opts.len(), 1);
        assert_eq!(c.island.form_opts.get("dots"), Some(&false));
        let j: DisplayConfig = serde_json::from_str(r#"{"island":{"skinOpts":{"neon.flicker":false}}}"#).unwrap();
        assert_eq!(j.island.skin_opts.get("neon.flicker"), Some(&false));
    }

    #[test]
    fn old_default_look_moves_to_clean_text() {
        let c: DisplayConfig = serde_json::from_str(
            r#"{"version":2,"island":{"skin":"dotMatrix","tint":"amber"},"pop":{"skin":"neon","tint":"ice"},"edge":{"skin":"nixie","tint":"amber"}}"#,
        )
        .unwrap();
        let c = c.sanitized();
        assert_eq!((c.island.skin.as_str(), c.island.tint.as_str()), ("clean", "white"));
        assert_eq!((c.pop.skin.as_str(), c.pop.tint.as_str()), ("neon", "ice"));
        assert_eq!(c.edge.skin, "clean");
        assert_eq!(c.version, VERSION);
    }

    #[test]
    fn removed_font_looks_become_options() {
        let c: DisplayConfig = serde_json::from_str(
            r#"{"version":3,"island":{"skin":"heavy","skinOpts":{"heavy.glow":true}},"pop":{"skin":"outline"},"edge":{"skin":"elegant","iconStyle":"nope","spacing":9}}"#,
        )
        .unwrap();
        let c = c.sanitized();
        assert_eq!(c.island.skin, "clean");
        assert_eq!(c.island.skin_opts.get("clean.bold"), Some(&true));
        assert_eq!(c.island.skin_opts.get("clean.glow"), Some(&true));
        assert_eq!(c.pop.skin_opts.get("clean.outline"), Some(&true));
        assert_eq!(c.edge.skin, "clean");
        assert_eq!(c.edge.icon_style, "tile");
        assert_eq!(c.edge.spacing, 3.0);
    }
}
