use crate::config::{AppConfig, Config};

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub glyph: String,
    pub subtitle: String,
    pub action: MenuAction,
    pub children: Vec<MenuItem>,
}

#[derive(Clone)]
pub enum MenuAction {
    None,
    Launch(AppConfig),
    Command(Vec<String>),
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

pub fn build(config: &Config, category: &str) -> Vec<MenuItem> {
    match category {
        "Home" => vec![item("deck-rs", "\u{f02dc}", "app launcher", MenuAction::None)],
        "Favorites" => {
            let mut favorites = config
                .ui
                .favorites
                .iter()
                .filter_map(|name| config.app.iter().find(|app| app.name.eq_ignore_ascii_case(name)))
                .map(app_item)
                .collect::<Vec<_>>();
            if favorites.is_empty() {
                favorites.push(item("no favorites configured", "\u{f04ce}", "", MenuAction::None));
            }
            favorites
        }
        "Settings" => settings(config),
        "Power" => power(config),
        name => config.app.iter().filter(|app| app.category.eq_ignore_ascii_case(name)).map(app_item).collect(),
    }
}

pub fn columns<'a>(root: &'a [MenuItem], rows: &[usize]) -> Vec<&'a [MenuItem]> {
    let mut columns = vec![root];
    for row in &rows[..rows.len() - 1] {
        match columns.last().and_then(|items| items.get(*row)) {
            Some(parent) if !parent.children.is_empty() => columns.push(&parent.children),
            _ => break,
        }
    }
    columns
}

fn app_item(app: &AppConfig) -> MenuItem {
    item(&app.name, &app.icon, "", MenuAction::Launch(app.clone()))
}

fn settings(config: &Config) -> Vec<MenuItem> {
    let group = |label: &str, glyph: &str, entries: &[(&str, &str, &str)]| MenuItem {
        children: entries.iter().map(|(label, glyph, key)| item(label, glyph, "", command(config, key))).collect(),
        ..item(label, glyph, "", MenuAction::None)
    };
    vec![
        group(
            "bluetooth",
            "\u{f00af}",
            &[
                ("on", "\u{f00af}", "bluetooth_on"),
                ("off", "\u{f00b2}", "bluetooth_off"),
                ("scan", "\u{f0349}", "bluetooth_scan"),
            ],
        ),
        group(
            "network",
            "\u{f05a9}",
            &[
                ("wi-fi on", "\u{f05a9}", "wifi_on"),
                ("wi-fi off", "\u{f05aa}", "wifi_off"),
                ("scan", "\u{f0349}", "wifi_scan"),
            ],
        ),
        group(
            "audio",
            "\u{f057e}",
            &[
                ("mute", "\u{f0581}", "audio_mute"),
                ("volume up", "\u{f057e}", "audio_volume_up"),
                ("volume down", "\u{f057e}", "audio_volume_down"),
            ],
        ),
    ]
}

fn power(config: &Config) -> Vec<MenuItem> {
    [
        ("suspend", "\u{f04b2}", "suspend"),
        ("power off", "\u{f0425}", "power_off"),
        ("reboot", "\u{f0709}", "reboot"),
        ("exit session", "\u{f0343}", "exit_session"),
    ]
    .into_iter()
    .map(|(label, glyph, key)| item(label, glyph, "", command(config, key)))
    .collect()
}

fn command(config: &Config, key: &str) -> MenuAction {
    config.commands.get(key).cloned().map_or(MenuAction::None, MenuAction::Command)
}

fn item(label: &str, glyph: &str, subtitle: &str, action: MenuAction) -> MenuItem {
    MenuItem { action, children: Vec::new(), glyph: glyph.into(), label: label.into(), subtitle: subtitle.into() }
}

pub fn category_icons(config: &Config, categories: &[String]) -> Vec<String> {
    categories.iter().map(|name| category_icon(config, name)).collect()
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
