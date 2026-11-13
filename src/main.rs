use iced::widget::{center, text};

fn main() -> iced::Result {
    iced::run(|_: &mut (), _: ()| {}, view)
}

fn view(_: &()) -> iced::Element<'_, ()> {
    center(text("hello world")).into()
}
