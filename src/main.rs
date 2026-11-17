mod actions;
mod config;
mod input_gamepad;
mod menu;
mod navigation;

slint::include_modules!();

use menu::{Menu, MenuAction};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

struct AppState {
    busy: bool,
    config: config::Config,
    current: Menu,
    category_index: usize,
    item_index: usize,
    error_message: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, config_error) = config::load();
    let category_names = menu::categories(&config);
    let category_index = category_names.iter().position(|name| name == "Favorites").unwrap_or(0);
    let current = menu::build(&config, &category_names[category_index]);
    let state = Rc::new(RefCell::new(AppState {
        busy: false,
        config,
        current,
        category_index,
        item_index: 0,
        error_message: config_error,
    }));
    let window = AppWindow::new()?;
    let weak_window = window.as_weak();
    let theme = window.global::<Theme>();
    theme.set_background(color(&state.borrow().config.ui.background, "#080b12"));
    theme.set_accent(color(&state.borrow().config.ui.accent, "#8ab4f8"));
    window.set_clock_text(clock_text().into());

    apply_config(&window, &state.borrow().config);
    apply_input_config(&window, &state.borrow().config.input.keyboard);
    update_window(&window, &state.borrow());

    let (sender, receiver) = mpsc::channel();
    let (action_sender, action_receiver) = mpsc::channel();
    let gamepad_action_sender = action_sender.clone();
    wire_callbacks(&window, state.clone(), action_sender);
    let input_config = state.borrow().config.input.clone();
    let gamepad_bindings = gamepad_bindings(&input_config.gamepad);
    input_gamepad::start(sender, gamepad_bindings, input_config.stick_threshold);
    let gamepad_state = state.clone();
    let gamepad_timer_window = weak_window.clone();
    let gamepad_timer = Timer::default();
    gamepad_timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        while let Ok(command) = receiver.try_recv() {
            if let Some(window) = gamepad_timer_window.upgrade()
                && window.window().is_visible()
            {
                if command == "select" {
                    activate_selected(&window, &gamepad_state, &gamepad_action_sender);
                } else {
                    navigate(&window, &gamepad_state, &command);
                }
            }
        }
        while let Ok(result) = action_receiver.try_recv() {
            if let Some(window) = gamepad_timer_window.upgrade() {
                let mut state = gamepad_state.borrow_mut();
                state.busy = false;
                if let Err(error) = result {
                    state.error_message = Some(error);
                }
                update_window(&window, &state);
            }
        }
    });

    let clock_window = weak_window;
    let clock_timer = Timer::default();
    clock_timer.start(TimerMode::Repeated, Duration::from_secs(30), move || {
        if let Some(window) = clock_window.upgrade() {
            window.set_clock_text(clock_text().into());
        }
    });

    window.window().set_fullscreen(true);
    window.run()?;
    Ok(())
}

fn wire_callbacks(window: &AppWindow, state: Rc<RefCell<AppState>>, action_sender: mpsc::Sender<Result<(), String>>) {
    let weak_window = window.as_weak();
    let navigation_state = state.clone();
    window.on_navigate(move |command| {
        if let Some(window) = weak_window.upgrade() {
            navigate(&window, &navigation_state, command.as_str());
        }
    });

    let weak_window = window.as_weak();
    let focus_state = state.clone();
    window.on_focus_item(move |index| {
        if let Some(window) = weak_window.upgrade() {
            let mut state = focus_state.borrow_mut();
            state.item_index = (index as usize).min(state.current.items.len().saturating_sub(1));
            update_window(&window, &state);
        }
    });

    let weak_window = window.as_weak();
    let selection_state = state.clone();
    let selection_sender = action_sender;
    window.on_select(move || {
        if let Some(window) = weak_window.upgrade() {
            activate_selected(&window, &selection_state, &selection_sender);
        }
    });

    let weak_window = window.as_weak();
    let scroll_state = state.clone();
    window.on_scroll_items(move |delta| {
        if let Some(window) = weak_window.upgrade() {
            navigate(&window, &scroll_state, if delta < 0.0 { "down" } else { "up" });
        }
    });

    let weak_window = window.as_weak();
    window.on_focus_category(move |index| {
        if let Some(window) = weak_window.upgrade() {
            let mut state = state.borrow_mut();
            state.category_index = (index as usize).min(state.current.categories.len().saturating_sub(1));
            let category = state.current.categories[state.category_index].clone();
            state.current = menu::build(&state.config, &category);
            state.item_index = 0;
            update_window(&window, &state);
        }
    });
}

fn apply_config(window: &AppWindow, config: &config::Config) {
    let theme = window.global::<Theme>();
    theme.set_accent(color(&config.ui.accent, "#8ab4f8"));
    theme.set_background(color(&config.ui.background, "#080b12"));
    theme.set_muted(color(&config.ui.muted, "#9198a8"));
    theme.set_panel(color(&config.ui.panel, "#171d29"));
    theme.set_selected(color(&config.ui.selected, "#252f40"));
    theme.set_text(color(&config.ui.text, "#f3f5f8"));
    theme.set_category_gap(config.ui.category_gap.max(120.0));
    theme.set_item_gap(config.ui.item_gap.max(0.0));
    theme.set_item_size(config.ui.item_size.clamp(16.0, 64.0));
    window.set_subtitle(config.ui.subtitle.clone().into());
}

fn clock_text() -> String {
    std::process::Command::new("date")
        .args(["+%H:%M"])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn apply_input_config(window: &AppWindow, bindings: &BTreeMap<String, String>) {
    let binding = |name: &str, default: &str| -> SharedString {
        let name = bindings.get(name).map_or(default, String::as_str);
        keyboard_key(name).unwrap_or_else(|| name.into())
    };
    window.set_key_back(binding("back", "Escape"));
    window.set_key_down(binding("down", "DownArrow"));
    window.set_key_left(binding("left", "LeftArrow"));
    window.set_key_menu(binding("menu", "Home"));
    window.set_key_right(binding("right", "RightArrow"));
    window.set_key_select(binding("select", "Enter"));
    window.set_key_up(binding("up", "UpArrow"));
}

fn keyboard_key(name: &str) -> Option<SharedString> {
    let key = match name {
        "Backspace" => slint::platform::Key::Backspace,
        "DownArrow" => slint::platform::Key::DownArrow,
        "Escape" => slint::platform::Key::Escape,
        "Home" => slint::platform::Key::Home,
        "LeftArrow" => slint::platform::Key::LeftArrow,
        "Menu" => slint::platform::Key::Menu,
        "Return" | "Enter" => slint::platform::Key::Return,
        "RightArrow" => slint::platform::Key::RightArrow,
        "Space" => slint::platform::Key::Space,
        "UpArrow" => slint::platform::Key::UpArrow,
        _ => return None,
    };
    Some(key.into())
}

fn gamepad_bindings(configured: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let defaults = [
        ("dpad_down", "down"),
        ("dpad_left", "left"),
        ("dpad_right", "right"),
        ("dpad_up", "up"),
        ("east", "back"),
        ("mode", "menu"),
        ("select", "back"),
        ("south", "select"),
        ("stick_down", "down"),
        ("stick_left", "left"),
        ("stick_right", "right"),
        ("stick_up", "up"),
        ("start", "menu"),
    ];
    defaults
        .into_iter()
        .map(|(key, value)| (key.into(), configured.get(key).cloned().unwrap_or_else(|| value.into())))
        .collect()
}

fn color(value: &str, fallback: &str) -> slint::Color {
    slint::Color::from_argb_encoded(
        parse_color(value)
            .unwrap_or_else(|| u32::from_str_radix(fallback.trim_start_matches('#'), 16).unwrap_or(0xff8ab4f8)),
    )
}

fn parse_color(value: &str) -> Option<u32> {
    let digits = value.strip_prefix('#')?;
    match digits.len() {
        6 => u32::from_str_radix(digits, 16).ok().map(|rgb| 0xff00_0000 | rgb),
        8 => u32::from_str_radix(digits, 16).ok(),
        _ => None,
    }
}

fn update_window(window: &AppWindow, state: &AppState) {
    let (labels, icons, values): (Vec<_>, Vec<_>, Vec<_>) = state
        .current
        .items
        .iter()
        .map(|item| (item.label.as_str().into(), item.glyph.as_str().into(), item.value.as_str().into()))
        .multiunzip();
    let categories: Vec<SharedString> = state.current.categories.iter().cloned().map(Into::into).collect();
    let category_icons: Vec<SharedString> = state.current.category_icons.iter().cloned().map(Into::into).collect();
    window.set_categories(ModelRc::new(VecModel::from(categories)));
    window.set_category_icons(ModelRc::new(VecModel::from(category_icons)));
    window.set_items(ModelRc::new(VecModel::from(labels)));
    window.set_item_icons(ModelRc::new(VecModel::from(icons)));
    window.set_item_values(ModelRc::new(VecModel::from(values)));
    window.set_selected_category(state.category_index as i32);
    window.set_selected_item(state.item_index as i32);
    window.set_error_message(state.error_message.clone().unwrap_or_default().into());
    window.set_busy(state.busy);
}

fn navigate(window: &AppWindow, state: &Rc<RefCell<AppState>>, command: &str) {
    if command == "select" {
        return;
    }
    let mut state = state.borrow_mut();
    match command {
        "left" | "right" => {
            let direction = if command == "left" { -1 } else { 1 };
            let count = state.current.categories.len();
            state.category_index = navigation::move_selection(state.category_index, count, direction);
            let category = state.current.categories[state.category_index].clone();
            state.current = menu::build(&state.config, &category);
            state.item_index = 0;
        }
        "up" | "down" => {
            let direction = if command == "up" { -1 } else { 1 };
            state.item_index = navigation::move_selection(state.item_index, state.current.items.len(), direction);
        }
        "back" | "menu" => {
            if let Some(index) = state.current.categories.iter().position(|category| category == "Favorites") {
                state.category_index = index;
                state.current = menu::build(&state.config, "Favorites");
                state.item_index = 0;
            }
        }
        _ => {}
    }
    update_window(window, &state);
}

fn activate_selected(
    window: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    action_sender: &mpsc::Sender<Result<(), String>>,
) {
    let action = {
        let state = state.borrow();
        state.current.items.get(state.item_index).map(|item| item.action.clone())
    };
    match action {
        Some(MenuAction::Launch(app)) => match actions::launch(app) {
            Ok(mut child) => {
                if let Err(error) = window.window().hide() {
                    show_error(window, state, error.to_string());
                    return;
                }
                let weak_window = window.as_weak();
                std::thread::spawn(move || {
                    let _ = child.wait();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak_window.upgrade() {
                            let _ = window.window().show();
                        }
                    });
                });
            }
            Err(error) => show_error(window, state, error),
        },
        Some(MenuAction::Command(command)) => {
            state.borrow_mut().busy = true;
            update_window(window, &state.borrow());
            let action_sender = action_sender.clone();
            std::thread::spawn(move || {
                let result = actions::run(command);
                let _ = action_sender.send(result);
            });
        }
        Some(MenuAction::None) | None => {}
    }
}

fn show_error(window: &AppWindow, state: &Rc<RefCell<AppState>>, error: String) {
    let mut state = state.borrow_mut();
    state.error_message = Some(error);
    update_window(window, &state);
}

trait MultiUnzip<A, B, C> {
    fn multiunzip(self) -> (Vec<A>, Vec<B>, Vec<C>);
}

impl<I, A, B, C> MultiUnzip<A, B, C> for I
where
    I: Iterator<Item = (A, B, C)>,
{
    fn multiunzip(self) -> (Vec<A>, Vec<B>, Vec<C>) {
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut third = Vec::new();
        for (a, b, c) in self {
            first.push(a);
            second.push(b);
            third.push(c);
        }
        (first, second, third)
    }
}
