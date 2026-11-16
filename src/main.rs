#![allow(non_camel_case_types, non_upper_case_globals)]

mod config;
mod deck;
mod input;
mod menu;
mod system;
mod view;

fn main() -> iced::Result {
    let (config, error) = config::configuration::load();
    deck::run(config, error)
}
