#![allow(non_camel_case_types, non_upper_case_globals)]

mod config;
mod deck;
mod input;
mod items;
mod modules;
mod status;
mod view;

fn main() -> iced::Result {
    let (config, error) = config::config::load();
    let path = Some(config.ui.font.clone()).filter(|font| !font.is_empty()).or_else(|| std::env::var("DECK_FONT").ok());
    let font = path
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_else(|| include_bytes!("../assets/SymbolsNerdFontMono-Regular.ttf").to_vec());
    iced::application(move || deck::deck::new(config.clone(), error.clone()), deck::deck::update, view::view)
        .font(font)
        .style(|_, _| iced::theme::Style { background_color: view::background, text_color: view::foreground })
        .subscription(deck::deck::subscription)
        .title("deck")
        .window(iced::window::Settings { fullscreen: true, ..Default::default() })
        .run()
}
