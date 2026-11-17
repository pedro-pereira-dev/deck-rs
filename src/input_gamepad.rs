use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

pub fn start(sender: Sender<String>, bindings: BTreeMap<String, String>, threshold: f32) {
    std::thread::Builder::new()
        .name("gamepad-input".into())
        .spawn(move || {
            let Ok(mut controllers) = gilrs::Gilrs::new() else {
                return;
            };
            let mut left_stick = (0_i8, 0_i8);
            loop {
                let Some(event) = controllers.next_event_blocking(None) else {
                    continue;
                };
                let Some(binding) = navigation_command(event.event, &mut left_stick, threshold) else {
                    continue;
                };
                let Some(command) = bindings.get(binding).cloned() else {
                    continue;
                };
                if sender.send(command).is_err() {
                    return;
                }
            }
        })
        .expect("could not start gamepad input thread");
}

fn navigation_command(event: gilrs::EventType, left_stick: &mut (i8, i8), threshold: f32) -> Option<&'static str> {
    match event {
        gilrs::EventType::ButtonPressed(button, _) => match button {
            gilrs::Button::DPadDown => Some("dpad_down"),
            gilrs::Button::DPadLeft => Some("dpad_left"),
            gilrs::Button::DPadRight => Some("dpad_right"),
            gilrs::Button::DPadUp => Some("dpad_up"),
            gilrs::Button::East => Some("east"),
            gilrs::Button::Mode => Some("mode"),
            gilrs::Button::Select => Some("select"),
            gilrs::Button::South => Some("south"),
            gilrs::Button::Start => Some("start"),
            _ => None,
        },
        gilrs::EventType::AxisChanged(axis, value, _) => {
            let threshold = threshold.clamp(0.1, 0.95);
            let direction = if value > threshold {
                1
            } else if value < -threshold {
                -1
            } else {
                0
            };
            let (previous, commands) = match axis {
                gilrs::Axis::LeftStickX => (&mut left_stick.0, ["stick_left", "stick_right"]),
                gilrs::Axis::LeftStickY => (&mut left_stick.1, ["stick_down", "stick_up"]),
                _ => return None,
            };
            (std::mem::replace(previous, direction) != direction && direction != 0)
                .then(|| commands[usize::from(direction > 0)])
        }
        _ => None,
    }
}
