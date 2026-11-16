use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

fn deck() -> super::deck {
    deck_with_snapshot(Arc::default())
}

fn deck_with_snapshot(system_snapshot: Arc<crate::model::status::system_snapshot>) -> super::deck {
    let mut config: crate::model::config::configuration =
        toml::from_str(include_str!("../../assets/config.defaults.toml")).unwrap();
    config.ui.categories = vec!["games".into()];
    let running = HashSet::new();
    let menu_items = super::build_configured_menu_items(&config, &system_snapshot, &running);

    super::deck {
        busy: None,
        category: 1,
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
fn navigation_clamps_categories_and_rows() {
    let mut deck = deck();
    drop(deck.apply_navigation(super::nav::down));
    assert_eq!(deck.selected_rows, [1]);
    drop(deck.apply_navigation(super::nav::up));
    assert_eq!(deck.selected_rows, [0]);
    drop(deck.apply_navigation(super::nav::up));
    assert_eq!(deck.selected_rows, [0]);
    drop(deck.apply_navigation(super::nav::left));
    assert_eq!(deck.category, 0);
    drop(deck.apply_navigation(super::nav::left));
    assert_eq!(deck.category, 0);
    drop(deck.apply_navigation(super::nav::right));
    assert_eq!(deck.category, 1);
    drop(deck.apply_navigation(super::nav::right));
    drop(deck.apply_navigation(super::nav::right));
    drop(deck.apply_navigation(super::nav::right));
    assert_eq!(deck.category, 4);
}

#[test]
fn selecting_a_settings_item_opens_its_submenu() {
    let system_snapshot = Arc::new(crate::model::status::system_snapshot {
        audio: Some(crate::model::services::audio::audio {
            muted: false,
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

    drop(deck.apply_navigation(super::nav::select));

    assert_eq!(deck.selected_rows, [1, 0]);
}

#[test]
fn menu_items_stay_static_after_later_system_snapshots() {
    let mut deck = deck();
    deck.menu_items.clear();
    drop(deck.update(super::message::snapshot(Arc::new(crate::model::status::system_snapshot {
        power_available: true,
        ..Default::default()
    }))));
    let initial_power_items = deck.menu_items[4].iter().map(|item| item.label.clone()).collect::<Vec<_>>();

    drop(deck.update(super::message::snapshot(Arc::new(crate::model::status::system_snapshot::default()))));

    let current_power_items = deck.menu_items[4].iter().map(|item| item.label.clone()).collect::<Vec<_>>();
    assert_eq!(initial_power_items, ["power off", "reboot", "exit session", "suspend"]);
    assert_eq!(current_power_items, initial_power_items);
}

#[test]
fn menu_navigation_returns_to_favorites() {
    let mut deck = deck();
    deck.category = 2;

    drop(deck.apply_navigation(super::nav::menu));

    assert_eq!(deck.category, 1);
}
