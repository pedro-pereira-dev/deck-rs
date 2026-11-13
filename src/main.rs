#![allow(non_camel_case_types, non_upper_case_globals)]

mod deck;
mod input;
mod items;
mod status;
mod sys;
mod view;

fn main() -> iced::Result {
    let mut app = iced::application(deck::deck::new, deck::deck::update, view::view)
        .style(|_, _| iced::theme::Style { background_color: view::background, text_color: view::foreground })
        .subscription(deck::deck::subscription)
        .title("deck")
        .window(iced::window::Settings { fullscreen: true, ..Default::default() });
    if let Some(bytes) = std::env::var("DECK_FONT").ok().and_then(|path| std::fs::read(path).ok()) {
        app = app.default_font(iced::Font::with_name("MesloLGS Nerd Font")).font(bytes);
    }
    app.run()
}
