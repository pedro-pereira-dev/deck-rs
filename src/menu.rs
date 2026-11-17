use crate::config::{AppConfig, Config};

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub glyph: String,
    pub value: String,
    pub action: MenuAction,
}

#[derive(Clone)]
pub enum MenuAction {
    None,
    Launch(AppConfig),
    Command(Vec<String>),
}

pub struct Menu {
    pub categories: Vec<String>,
    pub category_icons: Vec<String>,
    pub items: Vec<MenuItem>,
}

pub fn categories(config: &Config) -> Vec<String> {
    let mut categories: Vec<String> = vec!["Home".into(), "Favorites".into()];
    for category in config
        .app
        .iter()
        .map(|app| &app.category)
        .chain(config.ui.categories.iter())
        .filter(|category| !category.trim().is_empty())
    {
        if !categories.iter().any(|existing| existing.eq_ignore_ascii_case(category))
            && !["Settings", "Power"].iter().any(|name| name.eq_ignore_ascii_case(category))
        {
            categories.push(category.clone());
        }
    }
    categories.extend(["Settings".into(), "Power".into()]);
    categories
}

pub fn build(config: &Config, category: &str) -> Menu {
    let categories = categories(config);
    let category_icons = categories.iter().map(|name| category_icon(config, name)).collect();
    let items = match category {
        "Home" => vec![item("welcome", "\u{f02dc}", "", MenuAction::None)],
        "Favorites" => {
            let mut favorites = config
                .ui
                .favorites
                .iter()
                .filter_map(|name| config.app.iter().find(|app| app.name.eq_ignore_ascii_case(name)))
                .map(app_item)
                .collect::<Vec<_>>();
            if favorites.is_empty() {
                favorites.push(item("No favorites configured", "\u{f04ce}", "", MenuAction::None));
            }
            favorites
        }
        "Settings" => settings(config),
        "Power" => power(config),
        name => config.app.iter().filter(|app| app.category.eq_ignore_ascii_case(name)).map(app_item).collect(),
    };
    Menu { categories, category_icons, items }
}

fn app_item(app: &AppConfig) -> MenuItem {
    item(&app.name, &app.icon, "", MenuAction::Launch(app.clone()))
}

fn settings(config: &Config) -> Vec<MenuItem> {
    [
        ("Bluetooth on", "\u{f00af}", "bluetooth_on"),
        ("Bluetooth off", "\u{f00b2}", "bluetooth_off"),
        ("Scan Bluetooth devices", "\u{f0349}", "bluetooth_scan"),
        ("Wi-Fi on", "\u{f05a9}", "wifi_on"),
        ("Wi-Fi off", "\u{f05aa}", "wifi_off"),
        ("Scan Wi-Fi networks", "\u{f05a9}", "wifi_scan"),
        ("Mute audio", "\u{f0581}", "audio_mute"),
        ("Volume up", "\u{f057e}", "audio_volume_up"),
        ("Volume down", "\u{f057e}", "audio_volume_down"),
    ]
    .into_iter()
    .map(|(label, glyph, key)| item(label, glyph, "", command(config, key)))
    .collect()
}

fn power(config: &Config) -> Vec<MenuItem> {
    [
        ("Suspend", "\u{f04b2}", "suspend"),
        ("Power off", "\u{f0425}", "power_off"),
        ("Reboot", "\u{f0709}", "reboot"),
        ("Exit session", "\u{f0343}", "exit_session"),
    ]
    .into_iter()
    .map(|(label, glyph, key)| item(label, glyph, "", command(config, key)))
    .collect()
}

fn command(config: &Config, key: &str) -> MenuAction {
    config.commands.get(key).cloned().map_or(MenuAction::None, MenuAction::Command)
}

fn item(label: &str, glyph: &str, value: &str, action: MenuAction) -> MenuItem {
    MenuItem { action, glyph: glyph.into(), label: label.into(), value: value.into() }
}

fn category_icon(config: &Config, category: &str) -> String {
    let default = match category {
        "Favorites" => "\u{f04ce}",
        "Home" => "\u{f02dc}",
        "Power" => "\u{f0425}",
        "Settings" => "\u{f0493}",
        _ => "\u{f003b}",
    };
    config.ui.icons.get(category).cloned().unwrap_or_else(|| default.into())
}
