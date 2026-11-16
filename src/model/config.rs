use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Deserialize)]
pub struct configuration {
    #[serde(default)]
    pub app: Vec<configuration_app>,
    pub ui: configuration_ui,
}

#[derive(Clone, Deserialize)]
pub struct configuration_app {
    #[serde(default)]
    pub category: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub icon: String,
    pub name: String,
}

#[derive(Clone, Deserialize)]
pub struct configuration_ui {
    pub background: String,
    #[serde(default)]
    pub categories: Vec<String>,
    pub favorites: Vec<String>,
    pub font: String,
    pub font_family: String,
    #[serde(default)]
    pub icons: BTreeMap<String, String>,
    pub ping: String,
    pub navigation_sound: String,
}

const default_categories: [&str; 4] = ["Home", "Favorites", "Settings", "Power"];
const default_configuration_file_path: &str = include_str!("../../assets/config.defaults.toml");

fn get_path_from_environment_variable(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|value| !value.is_empty()).map(Into::into)
}

fn get_configuration_file_path() -> Option<PathBuf> {
    if let Some(path) = get_path_from_environment_variable("DECK_CONFIG") {
        return Some(path);
    }
    if let Some(config_home) = get_path_from_environment_variable("XDG_CONFIG_HOME") {
        return Some(config_home.join("deck-rs/config.toml"));
    }
    if let Some(home) = get_path_from_environment_variable("HOME") {
        return Some(home.join(".config/deck-rs/config.toml"));
    }
    None
}

fn merge_configurations(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge_configurations(existing, value),
                    None => drop(base.insert(key, value)),
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

impl configuration {
    pub fn load() -> (Self, Option<String>) {
        let mut default_configuration: toml::Value =
            toml::from_str(default_configuration_file_path).expect("default config");

        let mut error = None;
        if let Some(path) = get_configuration_file_path()
            && let Ok(contents) = std::fs::read_to_string(path)
        {
            match toml::from_str(&contents) {
                Ok(value) => merge_configurations(&mut default_configuration, value),
                Err(message) => error = Some(format!("config: {message}")),
            }
        }

        match default_configuration.try_into() {
            Ok(configuration) => (configuration, error),
            Err(message) => (
                toml::from_str(default_configuration_file_path).expect("default config"),
                Some(format!("config: {message}")),
            ),
        }
    }
}

impl configuration_ui {
    pub fn get_font(&self) -> Vec<u8> {
        (!self.font.is_empty())
            .then(|| std::fs::read(&self.font).ok())
            .flatten()
            .unwrap_or_else(|| include_bytes!("../../assets/SymbolsNerdFontMono-Regular.ttf").to_vec())
    }

    pub fn get_navigation_sound(&self) -> Vec<u8> {
        (!self.navigation_sound.is_empty())
            .then(|| std::fs::read(&self.navigation_sound).ok())
            .flatten()
            .unwrap_or_else(|| include_bytes!("../../assets/nav.wav").to_vec())
    }
}

pub fn get_configured_categories(config: &configuration) -> Vec<String> {
    let mut categories = vec![default_categories[0].into(), default_categories[1].into()];
    for category in &config.ui.categories {
        let is_default = default_categories.iter().any(|default| default.eq_ignore_ascii_case(category));
        let is_duplicate = categories.iter().any(|existing: &String| existing.eq_ignore_ascii_case(category));
        if !is_default && !is_duplicate {
            categories.push(category.clone());
        }
    }
    categories.extend(default_categories[2..].iter().map(|category| (*category).into()));
    categories
}

#[cfg(test)]
#[path = "config.test.rs"]
mod tests;
