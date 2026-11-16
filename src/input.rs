use crate::deck::{message, navigation};
use iced::futures::SinkExt;
use iced::keyboard;
use iced::{Event, event};

fn get_keyboard_navigation_from_controller(event: gilrs::EventType, stick: &mut (i8, i8)) -> Option<navigation> {
    use gilrs::EventType;
    match event {
        EventType::ButtonPressed(button, _) => navigate_with_controller_buttons(button),
        EventType::AxisChanged(axis, value, _) => navigate_with_controller_joystick(axis, value, stick),
        _ => None,
    }
}

fn navigate_with_controller_joystick(axis: gilrs::Axis, value: f32, stick: &mut (i8, i8)) -> Option<navigation> {
    use gilrs::Axis;
    let direction = if value > 0.6 {
        1
    } else if value < -0.6 {
        -1
    } else {
        0
    };
    let (previous, navs) = match axis {
        Axis::LeftStickX => (&mut stick.0, [navigation::left, navigation::right]),
        Axis::LeftStickY => (&mut stick.1, [navigation::down, navigation::up]),
        _ => return None,
    };
    let changed = std::mem::replace(previous, direction) != direction;
    (changed && direction != 0).then(|| navs[usize::from(direction > 0)])
}

fn navigate_with_controller_buttons(button: gilrs::Button) -> Option<navigation> {
    use gilrs::Button;
    match button {
        Button::DPadDown => Some(navigation::down),
        Button::DPadLeft => Some(navigation::left),
        Button::DPadRight => Some(navigation::right),
        Button::DPadUp => Some(navigation::up),
        Button::East | Button::Select => Some(navigation::back),
        Button::Mode | Button::Start => Some(navigation::menu),
        Button::South => Some(navigation::select),
        _ => None,
    }
}

pub fn handle_keyboard_event(event: Event, _: event::Status, _: iced::window::Id) -> Option<message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) = event else { return None };
    Some(message::key_press { key, text: text.map(|text| text.to_string()) })
}

pub fn handle_controller_events() -> impl iced::futures::Stream<Item = message> {
    iced::stream::channel(32, async |mut sender| {
        let _ = tokio::task::spawn_blocking(move || {
            let Ok(mut gilrs) = gilrs::Gilrs::new() else {
                return;
            };
            let mut stick = (0, 0);
            loop {
                let Some(nav) = gilrs
                    .next_event_blocking(None)
                    .and_then(|e| get_keyboard_navigation_from_controller(e.event, &mut stick))
                else {
                    continue;
                };
                if iced::futures::executor::block_on(sender.send(message::nav(nav))).is_err() {
                    return;
                }
            }
        })
        .await;
    })
}
