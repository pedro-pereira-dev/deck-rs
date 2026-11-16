use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

fn deck_with_apps() -> crate::controller::deck::deck {
    let mut config: crate::model::config::configuration =
        toml::from_str(include_str!("../../assets/config.defaults.toml")).unwrap();
    config.ui.categories = vec!["games".into()];
    config.ui.favorites = vec!["second".into(), "first".into(), "missing".into()];
    config.app = vec![
        crate::model::config::configuration_app {
            category: "games".into(),
            command: vec!["game-one".into()],
            env: BTreeMap::new(),
            icon: "one".into(),
            name: "first".into(),
        },
        crate::model::config::configuration_app {
            category: "games".into(),
            command: vec!["game-two".into()],
            env: BTreeMap::new(),
            icon: "two".into(),
            name: "second".into(),
        },
        crate::model::config::configuration_app {
            category: "tools".into(),
            command: vec!["tool".into()],
            env: BTreeMap::new(),
            icon: "tool".into(),
            name: "tool".into(),
        },
    ];
    let system_snapshot = Arc::default();
    let running = HashSet::new();
    let menu_items = crate::controller::deck::build_configured_menu_items(&config, &system_snapshot, &running);

    crate::controller::deck::deck {
        busy: None,
        category: 0,
        config: Arc::new(config),
        error: None,
        font_icons: iced::Font::DEFAULT,
        menu_items,
        selected_rows: vec![0],
        selected_row_by_category: vec![0; 5],
        running,
        animated_position: Vec::new(),
        last_animation_frame: None,
        last_pointer_activity: None,
        navigation_sound: PathBuf::new(),
        system_snapshot,
    }
}

#[test]
fn favorites_follow_configured_order_ignore_missing_apps_and_end_with_suspend() {
    let deck = deck_with_apps();
    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "Favorites");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["second", "first", "suspend"]);
}

#[test]
fn empty_favorites_show_placeholder_and_suspend() {
    let mut deck = deck_with_apps();
    Arc::make_mut(&mut deck.config).ui.favorites.clear();

    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "Favorites");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["-", "suspend"]);
    assert!(items[0].activation.is_none());
}

#[test]
fn app_pages_only_list_apps_from_that_category() {
    let deck = deck_with_apps();
    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "games");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["first", "second"]);
}

#[test]
fn app_pages_match_category_names_case_insensitively() {
    let deck = deck_with_apps();
    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "GAMES");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["first", "second"]);
}

#[test]
fn running_apps_are_marked_in_their_category() {
    let mut deck = deck_with_apps();
    deck.running.insert(1);

    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "games");

    assert_eq!(items[0].value, "");
    assert_eq!(items[1].value, "running");
}

#[test]
fn power_page_exposes_shutdown_controls_only_when_power_is_available() {
    let mut deck = deck_with_apps();
    let unavailable = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "Power");
    assert_eq!(unavailable.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["suspend"]);

    deck.system_snapshot =
        Arc::new(crate::model::status::system_snapshot { power_available: true, ..Default::default() });
    let available = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "Power");
    assert_eq!(
        available.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
        ["power off", "reboot", "exit session", "suspend"]
    );
}

#[test]
fn settings_only_show_controls_for_available_modules() {
    let mut deck = deck_with_apps();
    deck.system_snapshot = Arc::new(crate::model::status::system_snapshot {
        brightness: Some(50),
        power_profile: Some("balanced".into()),
        ..Default::default()
    });

    let items = super::build_category_items(&deck.config, &deck.system_snapshot, &deck.running, "Settings");
    let labels = items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>();

    assert_eq!(labels, ["network", "brightness", "power profile"]);
    assert_eq!(items[1].children.len(), 10);
    assert_eq!(items[2].children.len(), 3);
}
