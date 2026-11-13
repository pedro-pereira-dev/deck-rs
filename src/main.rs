use iced::widget::{button, center, column, text};

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
enum message {
    exit,
}

fn main() -> iced::Result {
    iced::application(|| (), update, view)
        .window(iced::window::Settings {
            fullscreen: true,
            ..Default::default()
        })
        .run()
}

fn update(_: &mut (), message: message) -> iced::Task<message> {
    match message {
        message::exit => iced::exit(),
    }
}

fn view(_: &()) -> iced::Element<'_, message> {
    center(column![text("hello world"), button("exit").on_press(message::exit)].spacing(10)).into()
}
