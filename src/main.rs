mod actions;
mod config;
mod input_gamepad;
mod menu;
mod navigation;

slint::include_modules!();

use menu::{MenuAction, MenuItem};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

struct AppState {
    busy: bool,
    cells: Rc<VecModel<Cell>>,
    categories: Vec<String>,
    config: config::Config,
    error_message: Option<String>,
    position: navigation::Position,
    root: Vec<MenuItem>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, config_error) = config::load();
    let categories = menu::categories(&config);
    let category = categories.iter().position(|name| name == "Favorites").unwrap_or(0);
    let state = Rc::new(RefCell::new(AppState {
        busy: false,
        cells: Rc::new(VecModel::default()),
        root: menu::build(&config, &categories[category]),
        categories,
        config,
        error_message: config_error,
        position: navigation::Position::new(category),
    }));
    let window = DeckWindow::new()?;
    let weak_window = window.as_weak();
    window.set_clock_date(clock_text("+%d/%m").into());
    window.set_clock_time(clock_text("+%H:%M").into());

    apply_config(&window, &state.borrow().config);
    apply_input_config(&window, &state.borrow().config.input.keyboard);
    {
        let state = state.borrow();
        let icons = menu::category_icons(&state.config, &state.categories);
        window.set_categories(strings(&state.categories));
        window.set_category_icons(strings(&icons));
        window.set_cells(ModelRc::from(state.cells.clone()));
    }
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
            window.set_clock_date(clock_text("+%d/%m").into());
            window.set_clock_time(clock_text("+%H:%M").into());
        }
    });

    window.window().set_fullscreen(true);
    window.run()?;
    Ok(())
}

fn wire_callbacks(window: &DeckWindow, state: Rc<RefCell<AppState>>, action_sender: mpsc::Sender<Result<(), String>>) {
    let weak_window = window.as_weak();
    let navigation_state = state.clone();
    window.on_navigate(move |command| {
        if let Some(window) = weak_window.upgrade() {
            navigate(&window, &navigation_state, command.as_str());
        }
    });

    let weak_window = window.as_weak();
    let selection_state = state.clone();
    let selection_sender = action_sender.clone();
    window.on_select(move || {
        if let Some(window) = weak_window.upgrade() {
            activate_selected(&window, &selection_state, &selection_sender);
        }
    });

    let weak_window = window.as_weak();
    let pick_state = state.clone();
    window.on_pick_cell(move |column, index| {
        if let Some(window) = weak_window.upgrade() {
            pick_state.borrow_mut().position.pick(column as usize, index as usize);
            activate_selected(&window, &pick_state, &action_sender);
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
            go_to_category(&window, &state, index as usize);
        }
    });
}

fn strings(values: &[String]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(values.iter().map(|value| value.as_str().into()).collect::<Vec<SharedString>>()))
}

fn apply_config(window: &DeckWindow, config: &config::Config) {
    let theme = window.global::<Theme>();
    theme.set_animation(config.ui.animation_ms as i64);
    theme.set_background(color(&config.ui.background, "#000000"));
    theme.set_font_family(config.ui.font_family.clone().into());
    theme.set_text(color(&config.ui.text, "#ffffff"));
    if let Ok(image) = slint::Image::load_from_path(std::path::Path::new(&config.ui.background_image)) {
        theme.set_background_image(image);
        theme.set_has_background_image(true);
    }
}

fn clock_text(format: &str) -> String {
    std::process::Command::new("date")
        .arg(format)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn apply_input_config(window: &DeckWindow, bindings: &BTreeMap<String, String>) {
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

fn update_window(window: &DeckWindow, state: &AppState) {
    let columns = menu::columns(&state.root, &state.position.rows);
    let depth = columns.len() - 1;
    let mut cells = Vec::new();
    for (column, items) in columns.iter().enumerate().skip(depth.saturating_sub(1)) {
        for (index, item) in items.iter().enumerate() {
            cells.push(Cell {
                active: column == depth,
                column: column as i32,
                focus: state.position.rows[column] as i32,
                glyph: item.glyph.as_str().into(),
                index: index as i32,
                label: item.label.as_str().into(),
                subtitle: item.subtitle.as_str().into(),
            });
        }
    }
    if state.cells.row_count() == cells.len() {
        for (row, cell) in cells.into_iter().enumerate() {
            state.cells.set_row_data(row, cell);
        }
    } else {
        state.cells.set_vec(cells);
    }
    window.set_busy(state.busy);
    window.set_category_title(state.categories[state.position.category].as_str().into());
    window.set_depth(depth as i32);
    window.set_error_message(state.error_message.clone().unwrap_or_default().into());
    window.set_selected_category(state.position.category as i32);
}

fn go_to_category(window: &DeckWindow, state: &Rc<RefCell<AppState>>, category: usize) {
    let mut state = state.borrow_mut();
    let category = category.min(state.categories.len() - 1);
    state.root = menu::build(&state.config, &state.categories[category]);
    state.position.go_to_category(category);
    update_window(window, &state);
}

fn navigate(window: &DeckWindow, state: &Rc<RefCell<AppState>>, command: &str) {
    let (depth, category_count, row_count, home, current) = {
        let state = state.borrow();
        let columns = menu::columns(&state.root, &state.position.rows);
        let home = state.categories.iter().position(|name| name == "Favorites").unwrap_or(0);
        (columns.len() - 1, state.categories.len(), columns[columns.len() - 1].len(), home, state.position.category)
    };
    match command {
        "left" | "right" if depth == 0 => {
            let mut target = navigation::Position::new(current);
            target.move_category(category_count, if command == "left" { -1 } else { 1 });
            go_to_category(window, state, target.category);
        }
        "left" | "back" if depth > 0 => {
            state.borrow_mut().position.go_back();
            update_window(window, &state.borrow());
        }
        "back" | "menu" => go_to_category(window, state, home),
        "up" | "down" => {
            state.borrow_mut().position.move_row(row_count, if command == "up" { -1 } else { 1 });
            update_window(window, &state.borrow());
        }
        _ => {}
    }
}

fn activate_selected(
    window: &DeckWindow,
    state: &Rc<RefCell<AppState>>,
    action_sender: &mpsc::Sender<Result<(), String>>,
) {
    let (action, has_children) = {
        let state = state.borrow();
        let columns = menu::columns(&state.root, &state.position.rows);
        let selected = columns.last().and_then(|items| items.get(state.position.rows[columns.len() - 1]));
        (selected.map(|item| item.action.clone()), selected.is_some_and(|item| !item.children.is_empty()))
    };
    if has_children {
        state.borrow_mut().position.enter();
        update_window(window, &state.borrow());
        return;
    }
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
                            window.window().set_fullscreen(true);
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
                let _ = action_sender.send(actions::run(command));
            });
        }
        Some(MenuAction::None) | None => {}
    }
}

fn show_error(window: &DeckWindow, state: &Rc<RefCell<AppState>>, error: String) {
    let mut state = state.borrow_mut();
    state.error_message = Some(error);
    update_window(window, &state);
}
