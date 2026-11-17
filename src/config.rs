use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub app: Vec<AppConfig>,
    #[serde(default)]
    pub commands: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub input: InputConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub gamepad: BTreeMap<String, String>,
    pub keyboard: BTreeMap<String, String>,
    pub stick_threshold: f32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self { gamepad: BTreeMap::new(), keyboard: BTreeMap::new(), stick_threshold: 0.6 }
    }
}

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub icon: String,
    pub name: String,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub accent: String,
    pub background: String,
    pub categories: Vec<String>,
    pub category_gap: f32,
    pub favorites: Vec<String>,
    pub font_family: String,
    pub icons: BTreeMap<String, String>,
    pub item_gap: f32,
    pub item_size: f32,
    pub muted: String,
    pub panel: String,
    pub selected: String,
    pub text: String,
    pub subtitle: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            accent: "#8ab4f8".into(),
            background: "#080b12".into(),
            categories: Vec::new(),
            category_gap: 270.0,
            favorites: Vec::new(),
            font_family: "Symbols Nerd Font Mono".into(),
            icons: BTreeMap::new(),
            item_gap: 18.0,
            item_size: 30.0,
            muted: "#9198a8".into(),
            panel: "#171d29".into(),
            selected: "#252f40".into(),
            text: "#f3f5f8".into(),
            subtitle: "deck-rs  /  app launcher".into(),
        }
    }
}

pub fn load() -> (Config, Option<String>) {
    let mut defaults: toml::Value =
        toml::from_str(include_str!("../assets/config.defaults.toml")).expect("bundled defaults are valid TOML");
    let Some(path) = config_path() else {
        return (defaults.try_into().expect("bundled defaults match config"), None);
    };
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return (defaults.try_into().expect("bundled defaults match config"), None),
    };
    let overlay: toml::Value = match toml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            return (
                defaults.try_into().expect("bundled defaults match config"),
                Some(format!("Configuration error: {error}")),
            );
        }
    };
    let fallback = defaults.clone();
    merge(&mut defaults, overlay);
    match defaults.try_into() {
        Ok(config) => (config, None),
        Err(error) => {
            (fallback.try_into().expect("bundled defaults match config"), Some(format!("Configuration error: {error}")))
        }
    }
}

fn merge(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("DECK_CONFIG")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).map(|path| path.join("deck-rs/config.toml")))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from).map(|path| path.join(".config/deck-rs/config.toml")))
}
