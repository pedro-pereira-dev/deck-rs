use crate::items::{bluetooth_glyph, item, items, network_glyph};
use crate::status::{query, status};
use crate::sys::{self, app};
use iced::{Point, Subscription, Task, event, time};
use std::sync::Arc;
use std::time::{Duration, Instant};

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

#[derive(Clone, Copy, Debug)]
pub enum panel {
    network,
    bluetooth,
    settings,
    power,
}

impl panel {
    pub const all: [panel; 4] = [panel::network, panel::bluetooth, panel::settings, panel::power];

    pub fn glyph(self, status: &status) -> &'static str {
        match self {
            panel::bluetooth => bluetooth_glyph(status),
            panel::network => network_glyph(status),
            panel::power => "\u{f0425}",
            panel::settings => "\u{f0493}",
        }
    }
}

#[derive(Clone)]
pub enum message {
    device(usize),
    done(Arc<status>, Option<String>),
    launch(usize),
    moved(Point),
    nav(nav),
    open(panel),
    ping(Option<f32>),
    press(usize, usize),
    quit,
    run(String, Vec<String>),
    spin,
    status(Arc<status>),
    tick,
    updates(String),
}

pub struct deck {
    pub apps: Vec<app>,
    pub busy: Option<String>,
    components: sysinfo::Components,
    pub cpu: String,
    pub cursor: bool,
    device: Option<usize>,
    pub disk: String,
    disks: sysinfo::Disks,
    pub error: Option<(String, Instant)>,
    pub focus: (usize, usize),
    pub index: usize,
    pub info: String,
    pub items: Vec<item>,
    pub memory: String,
    moved: Instant,
    pub panel: Option<panel>,
    pub ping: Option<f32>,
    pointer: Option<Point>,
    pub spinner: usize,
    pub status: Arc<status>,
    system: sysinfo::System,
    ticks: u64,
    pub updates: String,
}

fn refresh() -> Task<message> {
    Task::perform(query(), |status| message::status(status.into()))
}

impl deck {
    pub fn new() -> (Self, Task<message>) {
        let mut deck = deck {
            apps: sys::apps(),
            busy: None,
            components: sysinfo::Components::new_with_refreshed_list(),
            cpu: String::new(),
            cursor: false,
            device: None,
            disk: String::new(),
            disks: sysinfo::Disks::new_with_refreshed_list(),
            error: None,
            focus: (0, 0),
            index: 0,
            info: sys::info(),
            items: Vec::new(),
            memory: String::new(),
            moved: Instant::now(),
            panel: None,
            ping: None,
            pointer: None,
            spinner: 0,
            status: Arc::default(),
            system: sysinfo::System::new(),
            ticks: 0,
            updates: String::new(),
        };
        deck.stats();
        let ping = Task::perform(sys::ping(), message::ping);
        (deck, Task::batch([refresh(), ping, Task::perform(sys::updates(), message::updates)]))
    }

    fn rebuild(&mut self) {
        self.items = self.panel.map(|panel| items(panel, &self.status, self.device)).unwrap_or_default();
        self.index = self.index.min(self.items.len().saturating_sub(1));
    }

    fn show(&mut self, panel: Option<panel>, device: Option<usize>) {
        (self.device, self.index, self.panel) = (device, 0, panel);
        self.rebuild();
    }

    pub fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::device(index) => self.show(self.panel, Some(index)),
            message::done(status, error) => {
                let busy = self.busy.take();
                self.error = error.zip(busy).map(|(error, label)| (format!("{label} failed: {error}"), Instant::now()));
                self.device = None;
                return self.update(message::status(status));
            }
            message::launch(index) => {
                if let Some(app) = self.apps.get(index) {
                    return Task::future(sys::launch(app.command.clone())).discard();
                }
            }
            message::moved(point) => {
                if self.pointer.is_some_and(|last| last != point) {
                    (self.cursor, self.moved) = (true, Instant::now());
                }
                self.pointer = Some(point);
            }
            message::nav(nav) => return self.navigate(nav),
            message::open(panel) => {
                self.show(Some(panel), None);
                return refresh();
            }
            message::ping(ping) => self.ping = ping,
            message::press(row, column) => {
                match self.panel {
                    Some(_) => self.index = column,
                    None => self.focus = (row, column),
                }
                return self.navigate(nav::select);
            }
            message::quit => return Task::future(sys::logout()).discard().chain(iced::exit()),
            message::run(label, args) if self.busy.is_none() => {
                self.busy = Some(label);
                return Task::perform(
                    async move {
                        let error = sys::failure(45, &args).await;
                        (query().await, error)
                    },
                    |(status, error)| message::done(status.into(), error),
                );
            }
            message::run(..) => {}
            message::spin => self.spinner += 1,
            message::status(status) => {
                self.status = status;
                self.rebuild();
            }
            message::tick => {
                self.ticks += 1;
                if self.cursor && self.moved.elapsed() > Duration::from_secs(3) {
                    self.cursor = false;
                }
                self.stats();
                let mut tasks = Vec::new();
                if self.ticks.is_multiple_of(3600) {
                    tasks.push(Task::perform(sys::updates(), message::updates));
                }
                if self.ticks.is_multiple_of(60) {
                    tasks.push(Task::perform(sys::ping(), message::ping));
                }
                if self.ticks.is_multiple_of(5) {
                    tasks.push(refresh());
                }
                return Task::batch(tasks);
            }
            message::updates(updates) => self.updates = updates,
        }
        Task::none()
    }

    fn navigate(&mut self, nav: nav) -> Task<message> {
        if self.panel.is_some() {
            let last = self.items.len().saturating_sub(1);
            let action = match nav {
                nav::back | nav::menu => {
                    match self.device {
                        Some(_) => self.show(self.panel, None),
                        None => self.show(None, None),
                    }
                    None
                }
                nav::down => {
                    self.index = (self.index + 1).min(last);
                    None
                }
                nav::left => self.items.get(self.index).and_then(|item| item.left.clone()),
                nav::right => self.items.get(self.index).and_then(|item| item.right.clone()),
                nav::select => self.items.get(self.index).and_then(|item| item.select.clone()),
                nav::up => {
                    self.index = self.index.saturating_sub(1);
                    None
                }
            };
            return action.map_or_else(Task::none, |message| self.update(message));
        }
        let lengths = [self.apps.len(), panel::all.len()];
        let (mut row, mut column) = self.focus;
        match nav {
            nav::back => {}
            nav::down => row = 1,
            nav::left => column = column.saturating_sub(1),
            nav::menu => return self.update(message::open(panel::power)),
            nav::right => column += 1,
            nav::select if row == 0 => return self.update(message::launch(column)),
            nav::select => return panel::all.get(column).map_or_else(Task::none, |&p| self.update(message::open(p))),
            nav::up => row = 0,
        }
        if lengths[row] == 0 {
            row = 1 - row;
        }
        self.focus = (row, column.min(lengths[row].saturating_sub(1)));
        Task::none()
    }

    fn stats(&mut self) {
        let gib = |bytes: u64| bytes as f64 / 1024f64.powi(3);
        if self.ticks.is_multiple_of(5) {
            self.system.refresh_cpu_usage();
            self.components.refresh(false);
            let names = ["package", "tctl", "k10temp", "coretemp", "cpu", "soc"];
            let temperature = self
                .components
                .iter()
                .filter_map(|component| Some((component.label().to_lowercase(), component.temperature()?)))
                .filter(|(_, value)| *value > 0.0)
                .min_by_key(|(label, _)| names.iter().position(|name| label.contains(name)).unwrap_or(names.len()))
                .map(|(_, value)| format!(" {value:.0}°C"))
                .unwrap_or_default();
            self.cpu = format!("cpu {:.0}%{temperature}", self.system.global_cpu_usage());
        }
        if self.ticks.is_multiple_of(20) {
            self.system.refresh_memory();
            let (used, total) = (self.system.used_memory(), self.system.total_memory());
            self.memory = format!("mem {:.1}/{:.1} GiB", gib(used), gib(total));
        }
        if self.ticks.is_multiple_of(30) {
            self.disks.refresh(false);
            let root = self.disks.iter().find(|disk| disk.mount_point() == std::path::Path::new("/"));
            let used = root.map_or(0, |disk| 100 - disk.available_space() * 100 / disk.total_space().max(1));
            self.disk = format!("disk {used}%");
        }
    }

    pub fn subscription(&self) -> Subscription<message> {
        let spin = match self.busy {
            Some(_) => time::every(Duration::from_millis(120)).map(|_| message::spin),
            None => Subscription::none(),
        };
        Subscription::batch([
            spin,
            event::listen_with(crate::input::keys),
            time::every(Duration::from_secs(1)).map(|_| message::tick),
            Subscription::run(crate::input::gamepad),
        ])
    }
}
