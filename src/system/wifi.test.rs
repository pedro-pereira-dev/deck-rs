#[test]
fn converts_signal_to_bars() {
    assert_eq!(super::signal(-3000), 3);
    assert_eq!(super::signal(-6000), 2);
    assert_eq!(super::signal(-9500), 1);
}

#[test]
fn clamps_signal_bars_and_preserves_step_boundaries() {
    for (strength, expected) in [(-12000, 0), (-10000, 0), (-7500, 1), (-5000, 2), (-2500, 3), (0, 4), (500, 4)] {
        assert_eq!(super::signal(strength), expected, "signal strength {strength}");
    }
}

#[test]
fn network_list_marks_the_connected_path_uses_fallback_names_and_caps_results() {
    let paths = (0..9)
        .map(|index| zbus::zvariant::OwnedObjectPath::try_from(format!("/network/{index}")).unwrap())
        .collect::<Vec<_>>();
    let connected = Some(paths[1].clone());
    let entries = paths.into_iter().map(|path| (path, -5000)).collect();

    let networks = super::networks(entries, connected, &Default::default());

    assert_eq!(networks.len(), 8);
    assert!(!networks[0].is_active);
    assert!(networks[1].is_active);
    assert_eq!(networks[1].ssid, "unknown");
    assert_eq!(networks[1].signal, 2);
}
