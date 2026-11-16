use crate::controller::messages::{message, nav};
use crate::model::action::action;
use crate::model::config::{configuration, configuration_app};
use crate::model::menu::{build_category_items, check, item, item_activation};
use crate::model::services::{self, audio, bluetooth, network, power};
use crate::model::status::{collect_system_snapshot, system_snapshot};
use crate::view;
use iced::{Font, Subscription, Task, event, time};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct deck {
    pub animated_position: Vec<f32>,
    pub busy: Option<String>,
    pub category: usize,
    pub config: Arc<configuration>,
    pub error: Option<String>,
    pub font_icons: Font,
    pub last_animation_frame: Option<Instant>,
    pub last_pointer_activity: Option<Instant>,
    pub menu_items: Vec<Vec<item>>,
    pub running: HashSet<usize>,
    pub selected_row_by_category: Vec<usize>,
    pub selected_rows: Vec<usize>,
    pub navigation_sound: PathBuf,
    pub system_snapshot: Arc<system_snapshot>,
}

fn get_category_favorites_index(config: &configuration) -> usize {
    crate::model::config::get_configured_categories(config)
        .iter()
        .position(|category| category == "Favorites")
        .unwrap_or(0)
}

impl deck {
    pub fn new(config: configuration, error: Option<String>) -> (Self, Task<message>) {
        let configuration_font_icons = Font::with_name(Box::leak(config.ui.font_family.clone().into_boxed_str()));
        let configuration_navigation_sound = std::env::temp_dir().join("deck-rs-nav.wav");
        let _ = std::fs::write(&configuration_navigation_sound, config.ui.get_navigation_sound());

        let selected_row_by_category = vec![0; crate::model::config::get_configured_categories(&config).len()];
        let deck = deck {
            animated_position: Vec::new(),
            busy: None,
            category: get_category_favorites_index(&config),
            config: config.into(),
            error,
            font_icons: configuration_font_icons,
            last_animation_frame: None,
            last_pointer_activity: None,
            menu_items: Vec::new(),
            navigation_sound: configuration_navigation_sound,
            running: HashSet::new(),
            selected_row_by_category,
            selected_rows: vec![0],
            system_snapshot: Arc::default(),
        };
        let task = deck.refresh_system_snapshot();
        (deck, task)
    }

    pub fn get_category_items(&self) -> &[item] {
        self.menu_items.get(self.category).map_or(&[], Vec::as_slice)
    }

    // follow selected rows through nested menus to produce the visible columns.
    pub fn visible_columns<'a>(&self, category_items: &'a [item]) -> Vec<&'a [item]> {
        let mut columns = vec![category_items];
        for &row in &self.selected_rows[..self.selected_rows.len() - 1] {
            match columns.last().and_then(|items| items.get(row)) {
                Some(item) if !item.children.is_empty() => columns.push(&item.children),
                _ => break,
            }
        }
        columns
    }

    // animate toward the current menu position instead of jumping between rows.
    pub fn display_position(&self) -> Vec<f32> {
        if self.animated_position.len() < 2 { self.target_position() } else { self.animated_position.clone() }
    }

    // subscribe to input, periodic status refreshes, idle timing, and animation frames.
    pub fn subscription(&self) -> Subscription<message> {
        Subscription::batch([
            event::listen_with(crate::controller::input::keys),
            time::every(Duration::from_secs(5)).map(|_| message::tick),
            if self
                .last_pointer_activity
                .is_some_and(|last_activity| last_activity.elapsed() < Duration::from_millis(4500))
            {
                time::every(Duration::from_millis(500)).map(|_| message::idle)
            } else {
                Subscription::none()
            },
            if self.animated_position == self.target_position() {
                Subscription::none()
            } else {
                time::every(Duration::from_millis(16)).map(|_| message::frame(Instant::now()))
            },
            Subscription::run(crate::controller::input::gamepad),
        ])
    }

    // apply one event and return any asynchronous work it starts.
    pub fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::done(error) => {
                let label = self.busy.take().unwrap_or_default();
                self.error = error.map(|error| format!("{label} failed: {error}"));
                return self.refresh_system_snapshot();
            }
            message::frame(now) => {
                let target_position = self.target_position();
                let elapsed = self.last_animation_frame.map_or(0.016, |last_frame| (now - last_frame).as_secs_f32());
                let step = 1.0 - (-elapsed / 0.03).exp();
                self.animated_position.truncate(target_position.len());
                self.animated_position.extend_from_slice(&target_position[self.animated_position.len()..]);
                for (position, target) in self.animated_position.iter_mut().zip(target_position) {
                    *position = if (target - *position).abs() < 0.002 {
                        target
                    } else {
                        *position + (target - *position) * step
                    };
                }
                self.last_animation_frame = Some(now);
            }
            message::exited(index) => drop(self.running.remove(&index)),
            message::launch(index) => {
                if let Some(app) = self.config.app.get(index).cloned()
                    && self.running.insert(index)
                {
                    return Task::perform(launch_configured_app(app), move |_| message::exited(index));
                }
            }
            message::nav(nav) => return self.apply_navigation(nav),
            message::pick { category_index, .. } if category_index != self.category => {
                return self.select_menu_position(category_index, vec![]);
            }
            message::pick { selected_rows, .. } if selected_rows == self.selected_rows => {
                return self.apply_navigation(nav::select);
            }
            message::pick { selected_rows, .. } => return self.select_menu_position(self.category, selected_rows),
            message::run(label, action) if self.busy.is_none() => {
                (self.busy, self.error) = (Some(label), None);
                return Task::perform(perform_action(action), |result| message::done(result.err()));
            }
            message::run(..) | message::idle => {}
            message::snapshot(system_snapshot) => {
                if self.menu_items.is_empty() {
                    self.menu_items = build_configured_menu_items(&self.config, &system_snapshot, &self.running);
                }
                self.system_snapshot = system_snapshot;
            }
            message::mouse => self.last_pointer_activity = Some(Instant::now()),
            message::tick => return self.refresh_system_snapshot(),
        }
        Task::none()
    }

    // encode category, menu depth, and selected rows as the animation destination.
    pub fn target_position(&self) -> Vec<f32> {
        let head = [self.category as f32, self.selected_rows.len() as f32 - 1.0];
        head.into_iter().chain(self.selected_rows.iter().map(|row| *row as f32)).collect()
    }

    // change the active category or nested selection and play the navigation sound.
    fn select_menu_position(&mut self, category: usize, selected_rows: Vec<usize>) -> Task<message> {
        let category = category.min(self.selected_row_by_category.len().saturating_sub(1));
        let selected_rows = if category == self.category {
            selected_rows
        } else {
            vec![self.selected_row_by_category.get(category).copied().unwrap_or(0)]
        };
        if (category, &selected_rows) == (self.category, &self.selected_rows) || selected_rows.is_empty() {
            return Task::none();
        }
        (self.category, self.selected_rows, self.last_animation_frame) = (category, selected_rows, None);
        self.selected_row_by_category[category] = self.selected_rows[0];
        Task::future(play_navigation_sound(self.navigation_sound.clone())).discard()
    }

    // translate directional/select commands into menu movement or item activation.
    fn apply_navigation(&mut self, nav: nav) -> Task<message> {
        let category_items = self.get_category_items().to_vec();
        let columns = self.visible_columns(&category_items);
        let mut selected_rows = self.selected_rows[..columns.len()].to_vec();
        let (depth, items) = (selected_rows.len() - 1, columns[columns.len() - 1]);
        let row = selected_rows[depth].min(items.len().saturating_sub(1));
        let item = items.get(row);
        selected_rows[depth] = row;
        let category = self.category;
        match nav {
            nav::back if depth > 0 => self.select_menu_position(category, selected_rows[..depth].to_vec()),
            nav::left => self.select_menu_position(category.saturating_sub(1), vec![]),
            nav::right => self.select_menu_position(category + 1, vec![]),
            nav::down => self.select_menu_position(
                category,
                [&selected_rows[..depth], &[(row + 1).min(items.len().saturating_sub(1))]].concat(),
            ),
            nav::menu => self.select_menu_position(get_category_favorites_index(&self.config), vec![]),
            nav::select if item.is_some_and(|item| !item.children.is_empty()) => {
                let children = item.map_or(&[][..], |item| &item.children);
                let row = children.iter().position(|item| item.value == check).unwrap_or(0);
                self.select_menu_position(category, [&selected_rows[..], &[row]].concat())
            }
            nav::select => item.and_then(|item| item.activation.clone()).map_or(Task::none(), |activation| {
                self.update(match activation {
                    item_activation::launch(index) => message::launch(index),
                    item_activation::run(label, action) => message::run(label, action),
                })
            }),
            nav::up => {
                self.select_menu_position(category, [&selected_rows[..depth], &[row.saturating_sub(1)]].concat())
            }
            _ => Task::none(),
        }
    }

    // query system modules and send the completed snapshot back as a message.
    fn refresh_system_snapshot(&self) -> Task<message> {
        Task::perform(collect_system_snapshot(self.config.clone()), |snapshot| message::snapshot(snapshot.into()))
    }
}

pub(crate) fn build_configured_menu_items(
    config: &configuration,
    system_snapshot: &system_snapshot,
    running: &HashSet<usize>,
) -> Vec<Vec<item>> {
    crate::model::config::get_configured_categories(config)
        .iter()
        .map(|category| build_category_items(config, system_snapshot, running, category))
        .collect()
}

pub fn run(config: configuration, error: Option<String>) -> iced::Result {
    let font = config.ui.get_font();
    iced::application(move || deck::new(config.clone(), error.clone()), deck::update, view::view)
        .font(font)
        .style(|_, _| iced::theme::Style { background_color: view::background, text_color: view::foreground })
        .subscription(deck::subscription)
        .title("deck")
        .window(iced::window::Settings { fullscreen: true, ..Default::default() })
        .run()
}

// route a menu action to the module that implements it.
async fn perform_action(action: action) -> Result<(), String> {
    match action {
        action::audio(args) => audio::run_audio_command(args).await,
        action::bluetooth(method, path) => bluetooth::perform_bluetooth_action(method, path).await,
        action::brightness(level) => power::set_brightness(level).await,
        action::network(method, path) => network::perform_wifi_action(method, path).await,
        action::power(method) => power::perform_power_action(method).await,
        action::profile(profile) => power::set_profile(profile).await,
    }
}

// play the bundled navigation sound with whichever supported player is installed.
async fn play_navigation_sound(sound: PathBuf) {
    let player = ["pw-play", "paplay"].into_iter().find(|player| services::is_executable_on_path(player));
    if let Some(Ok(mut child)) = player.map(|player| tokio::process::Command::new(player).arg(sound).spawn()) {
        let _ = child.wait().await;
    }
}

// launch a configured app, adding desktop-session hints when running on wayland.
async fn launch_configured_app(app: configuration_app) {
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

#[cfg(test)]
#[path = "deck.test.rs"]
mod tests;
