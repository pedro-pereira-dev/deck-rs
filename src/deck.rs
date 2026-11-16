use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::configuration;
use crate::menu::{build_category_items, icon_check, item, item_activation};
use crate::system;
use crate::system::action::action;
use crate::system::host::{collect_system_snapshot, system_snapshot};
use crate::view;
use iced::keyboard::{Key, key::Named};
use iced::{Font, Subscription, Task, event, time};

#[derive(Clone, Copy)]
pub enum navigation {
    back,
    down,
    left,
    menu,
    right,
    select,
    up,
}

#[derive(Clone)]
pub enum message {
    done(Option<String>),
    exited(usize),
    frame(Instant),
    idle,
    key_press { key: Key, text: Option<String> },
    launch(usize),
    mouse,
    nav(navigation),
    pick { category_index: usize, selected_rows: Vec<usize> },
    run(String, action),
    snapshot(Arc<system_snapshot>),
    tick,
    wifi_cancel,
    wifi_check,
    wifi_character(char),
    wifi_clear,
    wifi_delete,
    wifi_move(navigation),
    wifi_select_key,
    wifi_shift,
    wifi_submit,
}

#[derive(Default)]
pub struct slide {
    position: Vec<f32>,
    last_frame: Option<Instant>,
}

pub struct deck {
    pub busy_label: Option<String>,
    pub category: usize,
    pub config: Arc<configuration>,
    pub error_text: Option<String>,
    pub font_icons: Font,
    pub last_pointer_activity: Option<Instant>,
    pub menu_items: Vec<Vec<item>>,
    pub navigation_sound: PathBuf,
    pub remembered_rows: Vec<usize>,
    pub running_apps: HashSet<usize>,
    pub selected_rows: Vec<usize>,
    pub slide: slide,
    pub system_snapshot: Arc<system_snapshot>,
    pub wifi_prompt: Option<wifi_prompt>,
}

pub struct wifi_prompt {
    pub agent: system::wifi::credential_agent,
    pub network: String,
    pub passphrase: String,
    pub selected_key: usize,
    pub requested: bool,
    pub uppercase: bool,
}

pub const wifi_keys: &str = "abcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{};:',.<>/?\\|`~\" ";

pub fn valid_wifi_passphrase(passphrase: &str) -> bool {
    (8..=63).contains(&passphrase.len())
        || (passphrase.len() == 64 && passphrase.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn get_favorites_category_index(config: &configuration) -> usize {
    crate::config::get_configured_categories(config).iter().position(|category| category == "Favorites").unwrap_or(0)
}

fn get_category_selection_items(
    config: &configuration,
    system_snapshot: &system_snapshot,
    running_apps: &HashSet<usize>,
) -> Vec<Vec<item>> {
    crate::config::get_configured_categories(config)
        .iter()
        .map(|category| build_category_items(config, system_snapshot, running_apps, category))
        .collect()
}

fn get_animation_frame_interval(config: &configuration) -> Duration {
    let frequency = config.ui.animation_frequency.max(1) as u64;
    Duration::from_nanos((1_000_000_000 / frequency).max(1))
}

fn input_navigation(key: Named) -> Option<navigation> {
    Some(match key {
        Named::ArrowDown => navigation::down,
        Named::ArrowLeft => navigation::left,
        Named::ArrowRight => navigation::right,
        Named::ArrowUp => navigation::up,
        Named::Backspace | Named::BrowserBack | Named::Escape | Named::Exit | Named::GoBack => navigation::back,
        Named::BrowserHome | Named::ContextMenu | Named::Home | Named::Power | Named::Settings => navigation::menu,
        Named::Enter | Named::Select | Named::Space => navigation::select,
        _ => return None,
    })
}

impl slide {
    fn is_moving(&self, target: &[f32]) -> bool {
        self.position != target
    }

    fn get_position(&self, target: Vec<f32>) -> Vec<f32> {
        if self.position.len() < 2 { target } else { self.position.clone() }
    }

    fn advance_animation_frame(&mut self, target: &[f32], now: Instant, frame_interval: Duration) {
        let elapsed =
            self.last_frame.map_or(frame_interval.as_secs_f32(), |last_frame| (now - last_frame).as_secs_f32());
        let step = 1.0 - (-elapsed / 0.03).exp();
        self.position.truncate(target.len());
        self.position.extend_from_slice(&target[self.position.len()..]);
        for (position, target) in self.position.iter_mut().zip(target) {
            *position =
                if (target - *position).abs() < 0.002 { *target } else { *position + (*target - *position) * step };
        }
        self.last_frame = Some(now);
    }
}

impl deck {
    fn new(config: configuration, error: Option<String>) -> (Self, Task<message>) {
        let configuration_font_icons = Font::with_name(Box::leak(config.ui.font_family.clone().into_boxed_str()));
        let configuration_navigation_sound = std::env::temp_dir().join("deck-rs-nav.wav");
        let _ = std::fs::write(&configuration_navigation_sound, config.ui.get_navigation_sound());

        let remembered_rows = vec![0; crate::config::get_configured_categories(&config).len()];
        let deck = deck {
            busy_label: None,
            category: get_favorites_category_index(&config),
            config: config.into(),
            error_text: error,
            font_icons: configuration_font_icons,
            last_pointer_activity: None,
            menu_items: Vec::new(),
            navigation_sound: configuration_navigation_sound,
            remembered_rows,
            running_apps: HashSet::new(),
            selected_rows: vec![0],
            slide: slide::default(),
            system_snapshot: Arc::default(),
            wifi_prompt: None,
        };
        let task = deck.refresh();
        (deck, task)
    }

    fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::done(error) => {
                let label = self.busy_label.take().unwrap_or_default();
                self.error_text = error.map(|error| format!("{label} failed: {error}"));
                self.wifi_prompt = None;
                return self.refresh();
            }
            message::frame(now) => {
                let target_position = self.get_menu_display_position();
                self.slide.advance_animation_frame(&target_position, now, get_animation_frame_interval(&self.config));
            }
            message::exited(index) => {
                self.running_apps.remove(&index);
                self.reset();
            }
            message::launch(index) => {
                if let Some(app) = self.config.app.get(index).cloned()
                    && self.running_apps.insert(index)
                {
                    self.reset();
                    return Task::perform(system::process::launch_app(app), move |_| message::exited(index));
                }
            }
            message::nav(nav) if self.wifi_prompt.is_some() => {
                if !self.wifi_prompt.as_ref().is_some_and(|prompt| prompt.requested) {
                    return Task::none();
                }
                return match nav {
                    navigation::select => self.update(message::wifi_select_key),
                    navigation::back => self.update(message::wifi_cancel),
                    navigation::up | navigation::down | navigation::left | navigation::right => {
                        self.update(message::wifi_move(nav))
                    }
                    navigation::menu => Task::none(),
                };
            }
            message::nav(nav) => return self.navigate(nav),
            message::key_press { key, text } if self.wifi_prompt.is_some() => {
                return self.update_wifi_prompt(key, text);
            }
            message::key_press { key: Key::Named(key), .. } => {
                if let Some(nav) = input_navigation(key) {
                    return self.navigate(nav);
                }
            }
            message::key_press { .. } => {}
            message::pick { category_index, .. } if category_index != self.category => {
                return self.update_selection(category_index, vec![]);
            }
            message::pick { selected_rows, .. } if selected_rows == self.selected_rows => {
                return self.navigate(navigation::select);
            }
            message::pick { selected_rows, .. } => return self.update_selection(self.category, selected_rows),
            message::run(label, action) if self.busy_label.is_none() => {
                if let action::wifi_connect(path, network) = action {
                    let agent = system::wifi::credential_agent::new();
                    self.wifi_prompt = Some(wifi_prompt {
                        agent: agent.clone(),
                        network,
                        passphrase: String::new(),
                        selected_key: 0,
                        requested: false,
                        uppercase: false,
                    });
                    self.busy_label = Some(label);
                    self.error_text = None;
                    return Task::perform(system::wifi::connect_with_agent(path, agent), |result| {
                        message::done(result.err())
                    });
                }
                (self.busy_label, self.error_text) = (Some(label), None);
                return Task::perform(system::perform_action(action), |result| message::done(result.err()));
            }
            message::wifi_check => {
                if let Some(prompt) = &mut self.wifi_prompt {
                    prompt.requested = prompt.agent.request_state() == system::wifi::agent_request::requested;
                }
            }
            message::wifi_character(value) => self.enter_wifi_character(value),
            message::wifi_delete => {
                if let Some(prompt) = &mut self.wifi_prompt {
                    prompt.passphrase.pop();
                }
            }
            message::wifi_clear => {
                if let Some(prompt) = &mut self.wifi_prompt {
                    prompt.passphrase.clear();
                }
            }
            message::wifi_shift => {
                if let Some(prompt) = &mut self.wifi_prompt {
                    prompt.uppercase = !prompt.uppercase;
                }
            }
            message::wifi_submit => {
                if let Some(prompt) = &mut self.wifi_prompt
                    && prompt.requested
                    && valid_wifi_passphrase(&prompt.passphrase)
                {
                    let agent = prompt.agent.clone();
                    let passphrase = std::mem::take(&mut prompt.passphrase);
                    prompt.requested = false;
                    return Task::perform(agent.submit(passphrase), |_| message::wifi_check);
                }
            }
            message::wifi_cancel => {
                if let Some(prompt) = &self.wifi_prompt {
                    let agent = prompt.agent.clone();
                    return Task::perform(agent.decline(), |_| message::wifi_check);
                }
            }
            message::wifi_move(nav) => self.move_wifi_key(nav),
            message::wifi_select_key => {
                if let Some(value) =
                    self.wifi_prompt.as_ref().and_then(|prompt| wifi_keys.chars().nth(prompt.selected_key))
                {
                    let value = self
                        .wifi_prompt
                        .as_ref()
                        .filter(|prompt| prompt.uppercase)
                        .map_or(value, |_| value.to_ascii_uppercase());
                    self.enter_wifi_character(value);
                }
            }
            message::run(..) | message::idle => {}
            message::snapshot(system_snapshot) => {
                self.system_snapshot = system_snapshot;
                self.reset();
            }
            message::mouse => self.last_pointer_activity = Some(Instant::now()),
            message::tick => return self.refresh(),
        }
        Task::none()
    }

    fn get_menu_display_position(&self) -> Vec<f32> {
        let head = [self.category as f32, self.selected_rows.len() as f32 - 1.0];
        head.into_iter().chain(self.selected_rows.iter().map(|row| *row as f32)).collect()
    }

    fn reset(&mut self) {
        self.menu_items = get_category_selection_items(&self.config, &self.system_snapshot, &self.running_apps);
        self.store_selection();
    }

    fn store_selection(&mut self) {
        for (index, items) in self.menu_items.iter().enumerate() {
            if let Some(row) = self.remembered_rows.get_mut(index) {
                *row = (*row).min(items.len().saturating_sub(1));
            }
        }
        let requested_rows = self.selected_rows.clone();
        let mut selected_rows = Vec::new();
        let mut items = self.get_category_items();
        for requested_row in requested_rows {
            let row = requested_row.min(items.len().saturating_sub(1));
            selected_rows.push(row);
            let Some(selected) = items.get(row) else { break };
            if selected.children.is_empty() {
                break;
            }
            items = &selected.children;
        }
        if selected_rows.is_empty() {
            selected_rows.push(0);
        }
        self.selected_rows = selected_rows;
        self.remembered_rows[self.category] = self.selected_rows[0];
    }

    fn update_selection(&mut self, category: usize, selected_rows: Vec<usize>) -> Task<message> {
        let category = category.min(self.remembered_rows.len().saturating_sub(1));
        let selected_rows = if category == self.category {
            selected_rows
        } else {
            vec![self.remembered_rows.get(category).copied().unwrap_or(0)]
        };
        if (category, &selected_rows) == (self.category, &self.selected_rows) || selected_rows.is_empty() {
            return Task::none();
        }
        if self.slide.position.is_empty() {
            self.slide.position = self.get_menu_display_position();
        }
        (self.category, self.selected_rows, self.slide.last_frame) = (category, selected_rows, None);
        self.remembered_rows[category] = self.selected_rows[0];
        Task::future(system::process::play_navigation_sound(self.navigation_sound.clone())).discard()
    }

    fn navigate(&mut self, nav: navigation) -> Task<message> {
        let (depth, selected_rows, row, item_count, child_row, activation) = {
            let columns = self.get_selection_category_breadcrumb(self.get_category_items());
            let mut selected_rows = self.selected_rows[..columns.len()].to_vec();
            let depth = selected_rows.len() - 1;
            let items = columns[depth];
            let row = selected_rows[depth].min(items.len().saturating_sub(1));
            let selected_item = items.get(row);
            selected_rows[depth] = row;
            let child_row = selected_item.and_then(|selected| {
                (!selected.children.is_empty())
                    .then(|| selected.children.iter().position(|item| item.value == icon_check).unwrap_or(0))
            });
            let activation = selected_item.and_then(|selected| selected.activation.clone());
            (depth, selected_rows, row, items.len(), child_row, activation)
        };
        let category = self.category;
        match nav {
            navigation::back if depth > 0 => self.update_selection(category, selected_rows[..depth].to_vec()),
            navigation::left => self.update_selection(category.saturating_sub(1), vec![]),
            navigation::right => self.update_selection(category + 1, vec![]),
            navigation::down => self.update_selection(
                category,
                [&selected_rows[..depth], &[(row + 1).min(item_count.saturating_sub(1))]].concat(),
            ),
            navigation::menu => self.update_selection(get_favorites_category_index(&self.config), vec![]),
            navigation::select if child_row.is_some() => {
                let row = child_row.unwrap_or_default();
                self.update_selection(category, [&selected_rows[..], &[row]].concat())
            }
            navigation::select => activation.map_or(Task::none(), |activation| {
                self.update(match activation {
                    item_activation::launch(index) => message::launch(index),
                    item_activation::run(label, action) => message::run(label, action),
                })
            }),
            navigation::up => {
                self.update_selection(category, [&selected_rows[..depth], &[row.saturating_sub(1)]].concat())
            }
            _ => Task::none(),
        }
    }

    fn update_wifi_prompt(&mut self, key: Key, text: Option<String>) -> Task<message> {
        if self.wifi_prompt.as_ref().is_some_and(|prompt| !prompt.requested) {
            return Task::none();
        }
        match key {
            Key::Named(Named::Escape | Named::BrowserBack | Named::GoBack | Named::Backspace) => {
                if matches!(key, Key::Named(Named::Backspace)) {
                    return self.update(message::wifi_delete);
                }
                return self.update(message::wifi_cancel);
            }
            Key::Named(Named::Space) => return self.update(message::wifi_character(' ')),
            Key::Named(Named::Enter | Named::Select) => return self.update(message::wifi_submit),
            Key::Named(Named::ArrowDown) => return self.update(message::wifi_move(navigation::down)),
            Key::Named(Named::ArrowUp) => return self.update(message::wifi_move(navigation::up)),
            Key::Named(Named::ArrowLeft) => return self.update(message::wifi_move(navigation::left)),
            Key::Named(Named::ArrowRight) => return self.update(message::wifi_move(navigation::right)),
            Key::Named(_) => {}
            Key::Character(_) => {
                if let Some(value) =
                    text.and_then(|text| (text.chars().count() == 1).then(|| text.chars().next()).flatten())
                {
                    return self.update(message::wifi_character(value));
                }
            }
            Key::Unidentified => {}
        }
        Task::none()
    }

    fn enter_wifi_character(&mut self, value: char) {
        if value.is_ascii()
            && !value.is_ascii_control()
            && let Some(prompt) = &mut self.wifi_prompt
            && prompt.passphrase.len() < 64
        {
            prompt.passphrase.push(value);
        }
    }

    fn move_wifi_key(&mut self, nav: navigation) {
        let Some(prompt) = &mut self.wifi_prompt else { return };
        let count = wifi_keys.chars().count();
        let columns = 12;
        prompt.selected_key = match nav {
            navigation::left => prompt.selected_key.saturating_sub(1),
            navigation::right => (prompt.selected_key + 1).min(count - 1),
            navigation::up => prompt.selected_key.saturating_sub(columns),
            navigation::down => (prompt.selected_key + columns).min(count - 1),
            _ => prompt.selected_key,
        };
    }

    fn refresh(&self) -> Task<message> {
        Task::perform(collect_system_snapshot(self.config.clone()), |snapshot| message::snapshot(snapshot.into()))
    }

    fn subscription(&self) -> Subscription<message> {
        let animation_frame_interval = get_animation_frame_interval(&self.config);
        Subscription::batch([
            event::listen_with(crate::input::handle_keyboard_event),
            time::every(Duration::from_secs(5)).map(|_| message::tick),
            if self
                .last_pointer_activity
                .is_some_and(|last_activity| last_activity.elapsed() < Duration::from_millis(4500))
            {
                time::every(Duration::from_millis(500)).map(|_| message::idle)
            } else {
                Subscription::none()
            },
            if !self.slide.is_moving(&self.get_menu_display_position()) {
                Subscription::none()
            } else {
                time::every(animation_frame_interval).map(|_| message::frame(Instant::now()))
            },
            Subscription::run(crate::input::handle_controller_events),
            if self
                .wifi_prompt
                .as_ref()
                .is_some_and(|prompt| prompt.agent.request_state() == system::wifi::agent_request::pending)
            {
                time::every(Duration::from_millis(100)).map(|_| message::wifi_check)
            } else {
                Subscription::none()
            },
        ])
    }

    pub(crate) fn get_selection_category_breadcrumb<'a>(&self, category_items: &'a [item]) -> Vec<&'a [item]> {
        let mut columns = vec![category_items];
        for &row in &self.selected_rows[..self.selected_rows.len() - 1] {
            match columns.last().and_then(|items| items.get(row)) {
                Some(item) if !item.children.is_empty() => columns.push(&item.children),
                _ => break,
            }
        }
        columns
    }

    pub(crate) fn get_display_position(&self) -> Vec<f32> {
        self.slide.get_position(self.get_menu_display_position())
    }

    pub(crate) fn get_category_items(&self) -> &[item] {
        self.menu_items.get(self.category).map_or(&[], Vec::as_slice)
    }
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
