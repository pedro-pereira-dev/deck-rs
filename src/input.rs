use crate::deck::{message, navigation};
use iced::futures::SinkExt;
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, event};

fn key_nav(key: Named) -> Option<navigation> {
    Some(match key {
        Named::ArrowDown => navigation::down,
        Named::ArrowLeft => navigation::left,
        Named::ArrowRight => navigation::right,
        Named::ArrowUp => navigation::up,
        Named::Backspace | Named::BrowserBack | Named::Escape | Named::Exit | Named::GoBack => navigation::back,
        Named::BrowserHome | Named::ContextMenu | Named::Home | Named::Power | Named::Settings => navigation::menu,
        Named::Enter | Named::Select | Named::Space => navigation::select,
        _ => return None,
    })
}

// convert d-pad buttons and joystick threshold crossings into navigation commands.
fn translate(event: gilrs::EventType, stick: &mut (i8, i8)) -> Option<navigation> {
    use gilrs::EventType;
    match event {
        EventType::ButtonPressed(button, _) => button_nav(button),
        EventType::AxisChanged(axis, value, _) => axis_nav(axis, value, stick),
        _ => None,
    }
}

fn axis_nav(axis: gilrs::Axis, value: f32, stick: &mut (i8, i8)) -> Option<navigation> {
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

fn button_nav(button: gilrs::Button) -> Option<navigation> {
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

// map named keyboard keys to the shared menu navigation commands.
pub fn keys(event: Event, _: event::Status, _: iced::window::Id) -> Option<message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key: Key::Named(key), .. }) = event else { return None };
    Some(message::nav(key_nav(key)?))
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

#[cfg(test)]
#[path = "input.test.rs"]
mod tests;
