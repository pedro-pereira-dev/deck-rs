use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

fn deck() -> super::deck {
    deck_with_snapshot(Arc::default())
}

fn deck_with_snapshot(system_snapshot: Arc<crate::system::host::system_snapshot>) -> super::deck {
    let mut config: crate::config::configuration =
        toml::from_str(include_str!("../assets/config.defaults.toml")).unwrap();
    config.ui.categories = vec!["games".into()];
    let running = HashSet::new();
    let menu_items = super::get_category_selection_items(&config, &system_snapshot, &running);

    super::deck {
        busy_label: None,
        category: 1,
        config: Arc::new(config),
        error_text: None,
        font_icons: iced::Font::DEFAULT,
        last_pointer_activity: None,
        menu_items,
        navigation_sound: PathBuf::new(),
        remembered_rows: vec![0; 5],
        running_apps: running,
        selected_rows: vec![0],
        slide: super::slide::default(),
        system_snapshot,
    }
}

#[test]
fn navigation_clamps_categories_and_rows() {
    let mut deck = deck();
    drop(deck.apply_navigation(super::navigation::down));
    assert_eq!(deck.selected_rows, [1]);
    drop(deck.apply_navigation(super::navigation::up));
    assert_eq!(deck.selected_rows, [0]);
    drop(deck.apply_navigation(super::navigation::up));
    assert_eq!(deck.selected_rows, [0]);
    drop(deck.apply_navigation(super::navigation::left));
    assert_eq!(deck.category, 0);
    drop(deck.apply_navigation(super::navigation::left));
    assert_eq!(deck.category, 0);
    drop(deck.apply_navigation(super::navigation::right));
    assert_eq!(deck.category, 1);
    drop(deck.apply_navigation(super::navigation::right));
    drop(deck.apply_navigation(super::navigation::right));
    drop(deck.apply_navigation(super::navigation::right));
    assert_eq!(deck.category, 4);
}

#[test]
fn selecting_a_settings_item_opens_its_submenu() {
    let system_snapshot = Arc::new(crate::system::host::system_snapshot {
        audio: Some(crate::system::audio::audio {
            is_muted: false,
            sinks: Vec::new(),
            sources: Vec::new(),
            tool: "wpctl",
            volume: 35,
        }),
        ..Default::default()
    });
    let mut deck = deck_with_snapshot(system_snapshot);
    deck.category = 3;
    deck.selected_rows = vec![1];

    drop(deck.apply_navigation(super::navigation::select));

    assert_eq!(deck.selected_rows, [1, 0]);
}

#[test]
fn menu_items_refresh_after_later_system_snapshots() {
    let mut deck = deck();
    deck.menu_items.clear();
    drop(deck.update(super::message::snapshot(Arc::new(crate::system::host::system_snapshot {
        is_power_available: true,
        ..Default::default()
    }))));
    let initial_power_items = deck.menu_items[4].iter().map(|item| item.label.clone()).collect::<Vec<_>>();

    drop(deck.update(super::message::snapshot(Arc::new(crate::system::host::system_snapshot::default()))));

    let current_power_items = deck.menu_items[4].iter().map(|item| item.label.clone()).collect::<Vec<_>>();
    assert_eq!(initial_power_items, ["power off", "reboot", "exit session", "suspend"]);
    assert_eq!(initial_power_items, ["power off", "reboot", "exit session", "suspend"]);
    assert_eq!(current_power_items, ["suspend"]);
}

#[test]
fn refreshed_menus_clamp_removed_items_and_submenus() {
    let system_snapshot = Arc::new(crate::system::host::system_snapshot {
        audio: Some(crate::system::audio::audio {
            is_muted: false,
            sinks: Vec::new(),
            sources: Vec::new(),
            tool: "wpctl",
            volume: 35,
        }),
        ..Default::default()
    });
    let mut deck = deck_with_snapshot(system_snapshot);
    deck.category = 3;
    deck.selected_rows = vec![1, 10];

    drop(deck.update(super::message::snapshot(Arc::default())));

    assert_eq!(deck.selected_rows, [0]);
    assert_eq!(deck.remembered_rows[deck.category], 0);
}

#[test]
fn menu_navigation_returns_to_favorites() {
    let mut deck = deck();
    deck.category = 2;

    drop(deck.apply_navigation(super::navigation::menu));

    assert_eq!(deck.category, 1);
}
