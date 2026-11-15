use crate::config::{app, config};
use crate::items::{check, item, items};
use crate::modules::{audio, bluetooth, network, power};
use crate::status::{query, status};
use iced::{Font, Subscription, Task, event, time};
use std::collections::HashSet;
use std::path::PathBuf;
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
    done(Option<String>),
    exited(usize),
    frame(Instant),
    idle,
    mouse,
    launch(usize),
    nav(nav),
    pick(usize, Vec<usize>),
    run(String, action),
    status(Arc<status>),
    tick,
}

pub struct deck {
    pub busy: Option<String>,
    pub category: usize,
    pub config: Arc<config>,
    pub error: Option<String>,
    pub icon: Font,
    pub path: Vec<usize>,
    pub rows: Vec<usize>,
    pub running: HashSet<usize>,
    pub last: Option<Instant>,
    pub moved: Option<Instant>,
    pub shown: Vec<f32>,
    pub sound: PathBuf,
    pub status: Arc<status>,
}

impl deck {
    pub fn new(config: config, error: Option<String>) -> (Self, Task<message>) {
        let icon = Font::with_name(Box::leak(config.ui.font_family.clone().into_boxed_str()));
        let sound = match config.ui.sound.as_str() {
            "" => {
                let path = std::env::temp_dir().join("deck-rs-nav.wav");
                let _ = std::fs::write(&path, include_bytes!("../assets/nav.wav"));
                path
            }
            path => path.into(),
        };
        let rows = vec![0; config.ui.modules.len()];
        let deck = deck {
            busy: None,
            category: favorites(&config),
            config: config.into(),
            error,
            icon,
            last: None,
            moved: None,
            path: vec![0],
            shown: Vec::new(),
            rows,
            running: HashSet::new(),
            sound,
            status: Arc::default(),
        };
        let task = deck.refresh();
        (deck, task)
    }

    pub fn root(&self) -> Vec<item> {
        self.config.ui.modules.get(self.category).map_or_else(Vec::new, |page| items(self, page))
    }

    pub fn columns<'a>(&self, root: &'a [item]) -> Vec<&'a [item]> {
        let mut columns = vec![root];
        for &row in &self.path[..self.path.len() - 1] {
            match columns.last().and_then(|items| items.get(row)) {
                Some(item) if !item.sub.is_empty() => columns.push(&item.sub),
                _ => break,
            }
        }
        columns
    }

    pub fn shown(&self) -> Vec<f32> {
        if self.shown.len() < 2 { self.target() } else { self.shown.clone() }
    }

    pub fn subscription(&self) -> Subscription<message> {
        Subscription::batch([
            event::listen_with(crate::input::keys),
            time::every(Duration::from_secs(5)).map(|_| message::tick),
            if self.moved.is_some_and(|moved| moved.elapsed() < Duration::from_millis(4500)) {
                time::every(Duration::from_millis(500)).map(|_| message::idle)
            } else {
                Subscription::none()
            },
            if self.shown == self.target() {
                Subscription::none()
            } else {
                time::every(Duration::from_millis(16)).map(|_| message::frame(Instant::now()))
            },
            Subscription::run(crate::input::gamepad),
        ])
    }

    pub fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::done(error) => {
                let label = self.busy.take().unwrap_or_default();
                self.error = error.map(|error| format!("{label} failed: {error}"));
                return self.refresh();
            }
            message::frame(now) => {
                let (target, elapsed) = (self.target(), self.last.map_or(0.016, |last| (now - last).as_secs_f32()));
                let step = 1.0 - (-elapsed / 0.03).exp();
                self.shown.truncate(target.len());
                self.shown.extend_from_slice(&target[self.shown.len()..]);
                for (shown, target) in self.shown.iter_mut().zip(target) {
                    *shown = if (target - *shown).abs() < 0.002 { target } else { *shown + (target - *shown) * step };
                }
                self.last = Some(now);
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
            message::pick(category, _) if category != self.category => return self.go(category, vec![]),
            message::pick(_, path) if path == self.path => return self.navigate(nav::select),
            message::pick(_, path) => return self.go(self.category, path),
            message::run(label, action) if self.busy.is_none() => {
                (self.busy, self.error) = (Some(label), None);
                return Task::perform(dispatch(action), |result| message::done(result.err()));
            }
            message::run(..) | message::idle => {}
            message::status(status) => self.status = status,
            message::mouse => self.moved = Some(Instant::now()),
            message::tick => return self.refresh(),
        }
        Task::none()
    }

    pub fn target(&self) -> Vec<f32> {
        let head = [self.category as f32, self.path.len() as f32 - 1.0];
        head.into_iter().chain(self.path.iter().map(|row| *row as f32)).collect()
    }

    fn go(&mut self, category: usize, path: Vec<usize>) -> Task<message> {
        let category = category.min(self.rows.len().saturating_sub(1));
        let path = if category == self.category { path } else { vec![self.rows.get(category).copied().unwrap_or(0)] };
        if (category, &path) == (self.category, &self.path) || path.is_empty() {
            return Task::none();
        }
        (self.category, self.path, self.last) = (category, path, None);
        self.rows[category] = self.path[0];
        Task::future(play(self.sound.clone())).discard()
    }

    fn navigate(&mut self, nav: nav) -> Task<message> {
        let root = self.root();
        let columns = self.columns(&root);
        let mut path = self.path[..columns.len()].to_vec();
        let (depth, items) = (path.len() - 1, columns[columns.len() - 1]);
        let row = path[depth].min(items.len().saturating_sub(1));
        let item = items.get(row);
        path[depth] = row;
        let category = self.category;
        match nav {
            nav::back if depth > 0 => self.go(category, path[..depth].to_vec()),
            nav::left => self.go(category.saturating_sub(1), vec![]),
            nav::right => self.go(category + 1, vec![]),
            nav::down => self.go(category, [&path[..depth], &[(row + 1).min(items.len().saturating_sub(1))]].concat()),
            nav::menu => self.go(favorites(&self.config), vec![]),
            nav::select if item.is_some_and(|item| !item.sub.is_empty()) => {
                let sub = item.map_or(&[][..], |item| &item.sub);
                let row = sub.iter().position(|item| item.value == check).unwrap_or(0);
                self.go(category, [&path[..], &[row]].concat())
            }
            nav::select => item.and_then(|item| item.select.clone()).map_or(Task::none(), |m| self.update(m)),
            nav::up => self.go(category, [&path[..depth], &[row.saturating_sub(1)]].concat()),
            _ => Task::none(),
        }
    }

    fn refresh(&self) -> Task<message> {
        Task::perform(query(self.config.clone()), |status| message::status(status.into()))
    }
}

fn favorites(config: &config) -> usize {
    config.ui.modules.iter().position(|page| page == "favorites").unwrap_or(0)
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

async fn play(sound: PathBuf) {
    let player = ["pw-play", "paplay"].into_iter().find(|player| crate::modules::which(player));
    if let Some(Ok(mut child)) = player.map(|player| tokio::process::Command::new(player).arg(sound).spawn()) {
        let _ = child.wait().await;
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
