#[test]
fn converts_hardware_brightness_to_percent() {
    assert_eq!(super::brightness_percent(50, 200), 25);
    assert_eq!(super::brightness_percent(200, 200), 100);
    assert_eq!(super::brightness_percent(300, 200), 100);
    assert_eq!(super::brightness_percent(0, 0), 0);
}

#[test]
fn converts_percent_to_clamped_hardware_level() {
    assert_eq!(super::brightness_level(200, 25), 50);
    assert_eq!(super::brightness_level(200, 100), 200);
    assert_eq!(super::brightness_level(200, 150), 200);
    assert_eq!(super::brightness_level(0, 50), 0);
}
