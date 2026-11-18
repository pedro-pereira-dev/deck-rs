use crate::config::{AppConfig, Config};

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub glyph: String,
    pub action: MenuAction,
}

#[derive(Clone)]
pub enum MenuAction {
    None,
    ExampleView,
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
        "Home" => vec![
            item("deck-rs", "\u{f02dc}", MenuAction::None),
            item("Example view", "\u{f06a9}", MenuAction::ExampleView),
        ],
        "Favorites" => {
            let mut favorites = config
                .ui
                .favorites
                .iter()
                .filter_map(|name| config.app.iter().find(|app| app.name.eq_ignore_ascii_case(name)))
                .map(app_item)
                .collect::<Vec<_>>();
            if favorites.is_empty() {
                favorites.push(item("no favorites configured", "\u{f04ce}", MenuAction::None));
            }
            favorites
        }
        "Settings" => vec![
            item("Bluetooth", "\u{f00af}", MenuAction::ExampleView),
            item("Network", "\u{f05a9}", MenuAction::ExampleView),
            item("Audio", "\u{f057e}", MenuAction::ExampleView),
        ],
        "Power" => power(config),
        name => config.app.iter().filter(|app| app.category.eq_ignore_ascii_case(name)).map(app_item).collect(),
    }
}

fn app_item(app: &AppConfig) -> MenuItem {
    item(&app.name, &app.icon, MenuAction::Launch(app.clone()))
}

fn power(config: &Config) -> Vec<MenuItem> {
    [
        ("power off", "\u{f0425}", "power_off"),
        ("reboot", "\u{f0709}", "reboot"),
        ("exit session", "\u{f0343}", "exit_session"),
        ("suspend", "\u{f04b2}", "suspend"),
    ]
    .into_iter()
    .map(|(label, glyph, key)| item(label, glyph, command(config, key)))
    .collect()
}

fn command(config: &Config, key: &str) -> MenuAction {
    config.commands.get(key).cloned().map_or(MenuAction::None, MenuAction::Command)
}

fn item(label: &str, glyph: &str, action: MenuAction) -> MenuItem {
    MenuItem { action, glyph: glyph.into(), label: label.into() }
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
