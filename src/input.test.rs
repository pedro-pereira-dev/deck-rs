use gilrs::Button;
use iced::keyboard::key::Named;

#[test]
fn maps_keyboard_navigation_and_ignores_unmapped_keys() {
    use crate::deck::navigation;
    assert!(matches!(super::key_nav(Named::ArrowUp), Some(navigation::up)));
    assert!(matches!(super::key_nav(Named::Escape), Some(navigation::back)));
    assert!(matches!(super::key_nav(Named::Home), Some(navigation::menu)));
    assert!(matches!(super::key_nav(Named::Enter), Some(navigation::select)));
    assert!(super::key_nav(Named::Tab).is_none());
}

#[test]
fn maps_gamepad_buttons_to_navigation() {
    use crate::deck::navigation;
    assert!(matches!(super::button_nav(Button::DPadDown), Some(navigation::down)));
    assert!(matches!(super::button_nav(Button::East), Some(navigation::back)));
    assert!(matches!(super::button_nav(Button::Start), Some(navigation::menu)));
    assert!(matches!(super::button_nav(Button::South), Some(navigation::select)));
    assert!(super::button_nav(Button::West).is_none());
}

#[test]
fn joystick_only_moves_when_crossing_threshold() {
    use crate::deck::navigation;
    let mut stick = (0, 0);
    assert!(super::axis_nav(gilrs::Axis::LeftStickX, 0.5, &mut stick).is_none());
    assert!(matches!(super::axis_nav(gilrs::Axis::LeftStickX, 0.8, &mut stick), Some(navigation::right)));
    assert!(super::axis_nav(gilrs::Axis::LeftStickX, 0.9, &mut stick).is_none());
    assert!(super::axis_nav(gilrs::Axis::LeftStickX, 0.0, &mut stick).is_none());
    assert!(matches!(super::axis_nav(gilrs::Axis::LeftStickX, -0.8, &mut stick), Some(navigation::left)));

    assert!(matches!(super::axis_nav(gilrs::Axis::LeftStickY, 0.8, &mut stick), Some(navigation::up)));
    assert!(matches!(super::axis_nav(gilrs::Axis::LeftStickY, -0.8, &mut stick), Some(navigation::down)));
}
