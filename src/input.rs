use crate::app::{message, nav};
use iced::futures::SinkExt;
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, event};

// map named keyboard keys to the shared menu navigation commands.
pub fn keys(event: Event, _: event::Status, _: iced::window::Id) -> Option<message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key: Key::Named(key), .. }) = event else { return None };
    Some(message::nav(key_nav(key)?))
}

fn key_nav(key: Named) -> Option<nav> {
    Some(match key {
        Named::ArrowDown => nav::down,
        Named::ArrowLeft => nav::left,
        Named::ArrowRight => nav::right,
        Named::ArrowUp => nav::up,
        Named::Backspace | Named::BrowserBack | Named::Escape | Named::Exit | Named::GoBack => nav::back,
        Named::BrowserHome | Named::ContextMenu | Named::Home | Named::Power | Named::Settings => nav::menu,
        Named::Enter | Named::Select | Named::Space => nav::select,
        _ => return None,
    })
}

// forward gamepad events from a blocking gilrs poller into iced's async stream.
pub fn gamepad() -> impl iced::futures::Stream<Item = message> {
    iced::stream::channel(32, async |mut sender| {
        let _ = tokio::task::spawn_blocking(move || {
            let Ok(mut gilrs) = gilrs::Gilrs::new() else {
                return;
            };
            let mut stick = (0, 0);
            loop {
                let Some(nav) = gilrs.next_event_blocking(None).and_then(|e| translate(e.event, &mut stick)) else {
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

// convert d-pad buttons and joystick threshold crossings into navigation commands.
fn translate(event: gilrs::EventType, stick: &mut (i8, i8)) -> Option<nav> {
    use gilrs::EventType;
    match event {
        EventType::ButtonPressed(button, _) => button_nav(button),
        EventType::AxisChanged(axis, value, _) => axis_nav(axis, value, stick),
        _ => None,
    }
}

fn axis_nav(axis: gilrs::Axis, value: f32, stick: &mut (i8, i8)) -> Option<nav> {
    use gilrs::Axis;
    let direction = if value > 0.6 {
        1
    } else if value < -0.6 {
        -1
    } else {
        0
    };
    let (previous, navs) = match axis {
        Axis::LeftStickX => (&mut stick.0, [nav::left, nav::right]),
        Axis::LeftStickY => (&mut stick.1, [nav::down, nav::up]),
        _ => return None,
    };
    let changed = std::mem::replace(previous, direction) != direction;
    (changed && direction != 0).then(|| navs[usize::from(direction > 0)])
}

fn button_nav(button: gilrs::Button) -> Option<nav> {
    use gilrs::Button;
    match button {
        Button::DPadDown => Some(nav::down),
        Button::DPadLeft => Some(nav::left),
        Button::DPadRight => Some(nav::right),
        Button::DPadUp => Some(nav::up),
        Button::East | Button::Select => Some(nav::back),
        Button::Mode | Button::Start => Some(nav::menu),
        Button::South => Some(nav::select),
        _ => None,
    }
}

#[cfg(test)]
#[path = "input.test.rs"]
mod tests;
