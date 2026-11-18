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
    pub animation_ms: u64,
    pub background: String,
    pub background_image: String,
    pub categories: Vec<String>,
    pub clock: ClockConfig,
    pub example: ExampleViewConfig,
    pub favorites: Vec<String>,
    pub font_family: String,
    pub header: HeaderConfig,
    pub icons: BTreeMap<String, String>,
    pub text: String,
    pub xmb: XmbConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            animation_ms: 180,
            background: "#000000".into(),
            background_image: String::new(),
            categories: Vec::new(),
            clock: ClockConfig::default(),
            example: ExampleViewConfig::default(),
            favorites: Vec::new(),
            font_family: "Symbols Nerd Font Mono".into(),
            header: HeaderConfig::default(),
            icons: BTreeMap::new(),
            text: "#ffffff".into(),
            xmb: XmbConfig::default(),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct ExampleViewConfig {
    pub back_button_font_percent: f32,
    pub back_button_height_percent: f32,
    pub back_button_left_percent: f32,
    pub back_button_top_percent: f32,
    pub back_button_width_percent: f32,
}

impl Default for ExampleViewConfig {
    fn default() -> Self {
        Self {
            back_button_font_percent: 3.5,
            back_button_height_percent: 8.0,
            back_button_left_percent: 40.0,
            back_button_top_percent: 46.0,
            back_button_width_percent: 20.0,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct HeaderConfig {
    pub category_font_percent: f32,
    pub date_font_percent: f32,
    pub date_top_percent: f32,
    pub left_padding_percent: f32,
    pub time_font_percent: f32,
    pub top_percent: f32,
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            category_font_percent: 6.5,
            date_font_percent: 1.8,
            date_top_percent: 17.25,
            left_padding_percent: 3.375,
            time_font_percent: 11.25,
            top_percent: 3.0,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct XmbConfig {
    pub category_center_percent: f32,
    pub category_center_y_percent: f32,
    pub category_gap_percent: f32,
    pub category_icon_height_percent: f32,
    pub category_icon_percent: f32,
    pub category_icon_width_percent: f32,
    pub category_selected_icon_percent: f32,
    pub error_bottom_percent: f32,
    pub error_font_percent: f32,
    pub error_horizontal_padding_percent: f32,
    pub item_column_offset_percent: f32,
    pub item_column_width_percent: f32,
    pub item_font_percent: f32,
    pub item_glyph_offset_percent: f32,
    pub item_glyph_percent: f32,
    pub item_glyph_width_percent: f32,
    pub item_height_percent: f32,
    pub item_label_offset_percent: f32,
    pub item_selected_font_percent: f32,
    pub item_selected_glyph_percent: f32,
    pub item_selected_height_percent: f32,
    pub item_selected_y_percent: f32,
    pub item_positive_near_step_percent: f32,
    pub item_positive_far_gap_percent: f32,
    pub item_positive_far_step_percent: f32,
    pub item_negative_near_step_percent: f32,
    pub item_negative_far_top_percent: f32,
    pub item_negative_far_step_percent: f32,
    pub selection_line_percent: f32,
    pub selection_line_offset_percent: f32,
    pub vertical_offset_percent: f32,
    pub viewport_top_percent: f32,
}

impl Default for XmbConfig {
    fn default() -> Self {
        Self {
            category_center_percent: 50.0,
            category_center_y_percent: 33.0,
            category_gap_percent: 7.875,
            category_icon_height_percent: 14.0,
            category_icon_percent: 5.0,
            category_icon_width_percent: 7.875,
            category_selected_icon_percent: 6.0,
            error_bottom_percent: 5.0,
            error_font_percent: 1.5,
            error_horizontal_padding_percent: 3.375,
            item_column_offset_percent: 5.625,
            item_column_width_percent: 25.3125,
            item_font_percent: 1.5,
            item_glyph_offset_percent: -2.8125,
            item_glyph_percent: 2.1,
            item_glyph_width_percent: 5.625,
            item_height_percent: 3.0,
            item_label_offset_percent: 2.8125,
            item_selected_font_percent: 2.25,
            item_selected_glyph_percent: 3.15,
            item_selected_height_percent: 4.5,
            item_selected_y_percent: 43.0,
            item_positive_near_step_percent: 7.5,
            item_positive_far_gap_percent: 2.5,
            item_positive_far_step_percent: 5.0,
            item_negative_near_step_percent: 20.0,
            item_negative_far_top_percent: 23.0,
            item_negative_far_step_percent: 5.0,
            selection_line_percent: 0.15,
            selection_line_offset_percent: 2.61,
            vertical_offset_percent: 0.0,
            viewport_top_percent: 33.0,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub box_height_percent: f32,
    pub box_width_percent: f32,
    pub date_color: String,
    pub date_font_percent: f32,
    pub date_format: String,
    pub font_family: String,
    pub format: String,
    pub horizontal_alignment: String,
    pub horizontal_padding_percent: f32,
    pub line_spacing_percent: f32,
    pub text_color: String,
    pub time_font_percent: f32,
    pub top_offset_percent: f32,
    pub vertical_alignment: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            box_height_percent: 0.0,
            box_width_percent: 100.0,
            date_color: "#999999".into(),
            date_font_percent: 1.8,
            date_format: "%A, %-d %B %Y".into(),
            font_family: "sans-serif".into(),
            format: "%H:%M".into(),
            horizontal_alignment: "center".into(),
            horizontal_padding_percent: 3.375,
            line_spacing_percent: 3.0,
            text_color: "#ffffff".into(),
            time_font_percent: 11.25,
            top_offset_percent: 3.0,
            vertical_alignment: "top".into(),
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
