use crate::config::{app, config};
use crate::items::{item, items};
use crate::modules::{audio, bluetooth, network, power};
use crate::status::{query, status};
use iced::{Font, Subscription, Task, event, time};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum nav {
    back,
    down,
    left,
    menu,
    right,
    select,
    up,
}

#[derive(Clone)]
pub enum action {
    audio(Vec<String>),
    bluetooth(&'static str, String),
    brightness(u32),
    network(&'static str, String),
    power(&'static str),
    profile(String),
}

#[derive(Clone)]
pub enum message {
    device(Option<usize>),
    done(Option<String>),
    exited(usize),
    launch(usize),
    nav(nav),
    press(bool, usize),
    run(String, action),
    status(Arc<status>),
    tick,
}

#[derive(Clone, Copy)]
pub enum page {
    app(usize),
    bluetooth,
    network,
    power,
    settings,
}

pub struct deck {
    pub busy: Option<String>,
    pub category: usize,
    pub config: config,
    pub device: Option<usize>,
    pub error: Option<String>,
    pub icon: Font,
    pub inside: bool,
    pub row: usize,
    pub running: HashSet<usize>,
    pub status: Arc<status>,
}

impl deck {
    pub fn new(config: config, error: Option<String>) -> (Self, Task<message>) {
        let icon = Font::with_name(Box::leak(config.ui.font_family.clone().into_boxed_str()));
        let deck = deck {
            busy: None,
            category: 0,
            config,
            device: None,
            error,
            icon,
            inside: false,
            row: 0,
            running: HashSet::new(),
            status: Arc::default(),
        };
        let task = deck.refresh();
        (deck, task)
    }

    pub fn items(&self) -> Vec<item> {
        items(self, self.pages()[self.category])
    }

    pub fn pages(&self) -> Vec<page> {
        let system = [page::network, page::bluetooth, page::settings, page::power];
        (0..self.config.app.len()).map(page::app).chain(system).collect()
    }

    pub fn subscription(&self) -> Subscription<message> {
        Subscription::batch([
            event::listen_with(crate::input::keys),
            time::every(Duration::from_secs(5)).map(|_| message::tick),
            Subscription::run(crate::input::gamepad),
        ])
    }

    pub fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::device(device) => (self.device, self.row) = (device, 0),
            message::done(error) => {
                let label = self.busy.take().unwrap_or_default();
                self.error = error.map(|error| format!("{label} failed: {error}"));
                return self.refresh();
            }
            message::exited(index) => drop(self.running.remove(&index)),
            message::launch(index) => {
                if let Some(app) = self.config.app.get(index).cloned()
                    && self.running.insert(index)
                {
                    return Task::perform(launch(app), move |_| message::exited(index));
                }
            }
            message::nav(nav) => return self.navigate(nav),
            message::press(true, row) => {
                (self.inside, self.row) = (true, row);
                return self.navigate(nav::select);
            }
            message::press(false, category) => self.focus(category),
            message::run(label, action) if self.busy.is_none() => {
                (self.busy, self.error) = (Some(label), None);
                return Task::perform(dispatch(action), |result| message::done(result.err()));
            }
            message::run(..) => {}
            message::status(status) => {
                self.status = status;
                self.row = self.row.min(self.items().len().saturating_sub(1));
            }
            message::tick => return self.refresh(),
        }
        Task::none()
    }

    fn focus(&mut self, category: usize) {
        self.category = category.min(self.pages().len() - 1);
        (self.device, self.inside, self.row) = (None, false, 0);
    }

    fn navigate(&mut self, nav: nav) -> Task<message> {
        let items = self.items();
        let item = items.get(self.row);
        let message = match (self.inside, nav) {
            (_, nav::menu) => {
                self.focus(usize::MAX);
                None
            }
            (false, nav::down) => {
                self.focus(self.category + 1);
                None
            }
            (false, nav::up) => {
                self.focus(self.category.saturating_sub(1));
                None
            }
            (false, nav::select) if matches!(self.pages()[self.category], page::app(_)) => {
                items.first().and_then(|item| item.select.clone())
            }
            (false, nav::right | nav::select) => {
                self.inside = !items.is_empty();
                None
            }
            (true, nav::down) => {
                self.row = (self.row + 1).min(items.len().saturating_sub(1));
                None
            }
            (true, nav::up) => {
                self.row = self.row.saturating_sub(1);
                None
            }
            (true, nav::left) if item.is_some_and(|item| item.left.is_some()) => {
                item.and_then(|item| item.left.clone())
            }
            (true, nav::right) => item.and_then(|item| item.right.clone()),
            (true, nav::select) => item.and_then(|item| item.select.clone()),
            (true, nav::back) if self.device.is_some() => Some(message::device(None)),
            (true, nav::back | nav::left) => {
                self.inside = false;
                None
            }
            _ => None,
        };
        message.map_or_else(Task::none, |message| self.update(message))
    }

    fn refresh(&self) -> Task<message> {
        Task::perform(query(self.config.clone()), |status| message::status(status.into()))
    }
}

async fn dispatch(action: action) -> Result<(), String> {
    match action {
        action::audio(args) => audio::run(args).await,
        action::bluetooth(method, path) => bluetooth::action(method, path).await,
        action::brightness(level) => power::set_brightness(level).await,
        action::network(method, path) => network::action(method, path).await,
        action::power(method) => power::action(method).await,
        action::profile(profile) => power::set_profile(profile).await,
    }
}

async fn launch(app: app) {
    let Some((program, args)) = app.command.split_first() else { return };
    let mut command = tokio::process::Command::new(program);
    command.args(args);
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        command.envs([
            ("ELECTRON_OZONE_PLATFORM_HINT", "wayland"),
            ("GDK_BACKEND", "wayland,x11"),
            ("NIXOS_OZONE_WL", "1"),
            ("QT_QPA_PLATFORM", "wayland;xcb"),
            ("SDL_VIDEODRIVER", "wayland,x11"),
            ("XDG_SESSION_TYPE", "wayland"),
        ]);
    }
    if let Ok(mut child) = command.envs(&app.env).spawn() {
        let _ = child.wait().await;
    }
}
