#![allow(non_camel_case_types, non_upper_case_globals)]

mod app;
mod config;
mod input;
mod menu;
mod system;
mod view;

fn main() -> iced::Result {
    let (config, error) = config::configuration::load();
    app::run(config, error)
}
