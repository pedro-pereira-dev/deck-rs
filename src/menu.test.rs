use std::collections::{BTreeMap, HashSet};

fn fixtures() -> (crate::config::configuration, crate::system::host::system_snapshot, HashSet<usize>) {
    let mut config: crate::config::configuration =
        toml::from_str(include_str!("../assets/config.defaults.toml")).unwrap();
    config.ui.categories = vec!["games".into()];
    config.ui.favorites = vec!["second".into(), "first".into(), "missing".into()];
    config.app = vec![
        crate::config::configuration_app {
            category: "games".into(),
            command: vec!["game-one".into()],
            env: BTreeMap::new(),
            icon: "one".into(),
            name: "first".into(),
        },
        crate::config::configuration_app {
            category: "games".into(),
            command: vec!["game-two".into()],
            env: BTreeMap::new(),
            icon: "two".into(),
            name: "second".into(),
        },
        crate::config::configuration_app {
            category: "tools".into(),
            command: vec!["tool".into()],
            env: BTreeMap::new(),
            icon: "tool".into(),
            name: "tool".into(),
        },
    ];
    (config, Default::default(), HashSet::new())
}

#[test]
fn favorites_follow_configured_order_ignore_missing_apps_and_end_with_suspend() {
    let (config, status, running) = fixtures();
    let items = super::build_category_items(&config, &status, &running, "Favorites");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["second", "first", "suspend"]);
}

#[test]
fn empty_favorites_show_placeholder_and_suspend() {
    let (mut config, status, running) = fixtures();
    config.ui.favorites.clear();

    let items = super::build_category_items(&config, &status, &running, "Favorites");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["-", "suspend"]);
    assert!(items[0].activation.is_none());
}

#[test]
fn app_pages_only_list_apps_from_that_category() {
    let (config, status, running) = fixtures();
    let items = super::build_category_items(&config, &status, &running, "games");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["first", "second"]);
}

#[test]
fn app_pages_match_category_names_case_insensitively() {
    let (config, status, running) = fixtures();
    let items = super::build_category_items(&config, &status, &running, "GAMES");

    assert_eq!(items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["first", "second"]);
}

#[test]
fn running_apps_are_marked_in_their_category() {
    let (config, status, mut running) = fixtures();
    running.insert(1);

    let items = super::build_category_items(&config, &status, &running, "games");

    assert_eq!(items[0].value, "");
    assert_eq!(items[1].value, "running");
}

#[test]
fn power_page_exposes_shutdown_controls_only_when_power_is_available() {
    let (config, status, running) = fixtures();
    let unavailable = super::build_category_items(&config, &status, &running, "Power");
    assert_eq!(unavailable.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), ["suspend"]);

    let available_status = crate::system::host::system_snapshot { is_power_available: true, ..Default::default() };
    let available = super::build_category_items(&config, &available_status, &running, "Power");
    assert_eq!(
        available.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(),
        ["power off", "reboot", "exit session", "suspend"]
    );
}

#[test]
fn settings_only_show_controls_for_available_modules() {
    let (config, _, running) = fixtures();
    let status = crate::system::host::system_snapshot {
        brightness: Some(50),
        power_profile: Some("balanced".into()),
        ..Default::default()
    };

    let items = super::build_category_items(&config, &status, &running, "Settings");
    let labels = items.iter().map(|item| item.label.as_str()).collect::<Vec<_>>();

    assert_eq!(labels, ["network", "brightness", "power profile"]);
    assert_eq!(items[1].children.len(), 10);
    assert_eq!(items[2].children.len(), 3);
}
