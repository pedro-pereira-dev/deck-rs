#![allow(non_camel_case_types, non_upper_case_globals)]

mod controller;
mod model;
mod view;

fn main() -> iced::Result {
    let (config, error) = model::config::configuration::load();
    controller::deck::run(config, error)
}
