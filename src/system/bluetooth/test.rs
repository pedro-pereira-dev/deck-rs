#[test]
fn connected_state_takes_precedence_over_paired() {
    assert!(matches!(super::device_state(Some(true), Some(true)), super::state::connected));
    assert!(matches!(super::device_state(Some(true), Some(false)), super::state::connected));
}

#[test]
fn paired_and_new_states_are_detected() {
    assert!(matches!(super::device_state(Some(false), Some(true)), super::state::paired));
    assert!(matches!(super::device_state(None, None), super::state::new));
    assert!(matches!(super::device_state(Some(false), Some(false)), super::state::new));
}

#[test]
fn device_name_falls_back_to_address() {
    assert_eq!(super::device_name(Some("headphones".into()), Some("00:11".into())), Some("headphones".into()));
    assert_eq!(super::device_name(None, Some("00:11".into())), Some("00:11".into()));
    assert_eq!(super::device_name(None, None), None);
}

#[test]
fn orders_devices_by_state_and_limits_the_list() {
    let mut devices: Vec<super::device> = (0..12)
        .map(|index| super::device {
            battery: None,
            name: format!("device-{index}"),
            object_path: format!("/device/{index}"),
            state: if index == 0 { super::state::connected } else { super::state::new },
        })
        .collect();

    super::order_devices(&mut devices);

    assert_eq!(devices.len(), 10);
    assert!(matches!(devices[0].state, super::state::connected));
    assert!(devices.iter().all(|device| matches!(device.state, super::state::connected | super::state::new)));
}
