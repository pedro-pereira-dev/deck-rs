use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Deserialize)]
pub struct app {
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub icon: String,
    pub name: String,
}

#[derive(Clone, Deserialize)]
pub struct backend {
    pub backend: String,
}

#[derive(Clone, Deserialize)]
pub struct ui {
    pub font: String,
    pub font_family: String,
}

#[derive(Clone, Deserialize)]
pub struct config {
    pub app: Vec<app>,
    pub audio: backend,
    pub bluetooth: backend,
    pub brightness: backend,
    pub network: backend,
    pub power: backend,
    pub profile: backend,
    pub ui: ui,
}

const defaults: &str = include_str!("../config.toml");

impl config {
    pub fn load() -> (Self, Option<String>) {
        let mut merged: toml::Value = toml::from_str(defaults).expect("default config");
        let mut error = None;
        let user = env("DECK_CONFIG").or_else(|| {
            Some(env("XDG_CONFIG_HOME").or_else(|| Some(env("HOME")?.join(".config")))?.join("deck-rs/config.toml"))
        });
        for path in [env("DECK_DEFAULT_CONFIG"), user].into_iter().flatten() {
            let Ok(contents) = std::fs::read_to_string(&path) else { continue };
            match toml::from_str(&contents) {
                Ok(value) => merge(&mut merged, value),
                Err(message) => error = Some(format!("config: {message}")),
            }
        }
        match merged.try_into() {
            Ok(config) => (config, error),
            Err(message) => (toml::from_str(defaults).expect("default config"), Some(format!("config: {message}"))),
        }
    }
}

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|value| !value.is_empty()).map(Into::into)
}

fn merge(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base), toml::Value::Table(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => drop(base.insert(key, value)),
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn overrides_merge_over_defaults() {
        let mut merged: toml::Value = toml::from_str(super::defaults).unwrap();
        let overlay = toml::toml! {
            app = [{ name = "custom", command = ["custom"] }]
            network.backend = "none"
        };
        super::merge(&mut merged, toml::Value::Table(overlay));
        let config: super::config = merged.try_into().unwrap();
        assert_eq!(config.app.len(), 1);
        assert_eq!(config.app[0].name, "custom");
        assert_eq!(config.network.backend, "none");
        assert_eq!(config.audio.backend, "auto");
    }
}
