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
    launch(usize),
    mouse,
    nav(navigation),
    pick { category_index: usize, selected_rows: Vec<usize> },
    run(String, action),
    snapshot(Arc<system_snapshot>),
    tick,
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
        };
        let task = deck.refresh();
        (deck, task)
    }

    fn update(&mut self, message: message) -> Task<message> {
        match message {
            message::done(error) => {
                let label = self.busy_label.take().unwrap_or_default();
                self.error_text = error.map(|error| format!("{label} failed: {error}"));
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
            message::nav(nav) => return self.navigate(nav),
            message::pick { category_index, .. } if category_index != self.category => {
                return self.update_selection(category_index, vec![]);
            }
            message::pick { selected_rows, .. } if selected_rows == self.selected_rows => {
                return self.navigate(navigation::select);
            }
            message::pick { selected_rows, .. } => return self.update_selection(self.category, selected_rows),
            message::run(label, action) if self.busy_label.is_none() => {
                (self.busy_label, self.error_text) = (Some(label), None);
                return Task::perform(system::perform_action(action), |result| message::done(result.err()));
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
