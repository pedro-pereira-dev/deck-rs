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
    example_view: bool,
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
        example_view: false,
        position: navigation::Position::new(category),
    }));
    let window = DeckWindow::new()?;
    let weak_window = window.as_weak();
    let clock_config = state.borrow().config.ui.clock.clone();
    window.set_clock_date(clock_text(&clock_config.date_format).into());
    window.set_clock_time(clock_text(&clock_config.format).into());

    apply_config(&window, &state.borrow().config);
    apply_input_config(&window, &state.borrow().config.input.keyboard);
    {
        let state = state.borrow();
        let icons = menu::category_icons(&state.config, &state.categories);
        let categories =
            state.categories.iter().zip(icons).map(|(_, glyph)| Category { glyph: glyph.into() }).collect::<Vec<_>>();
        window.set_categories(ModelRc::new(VecModel::from(categories)));
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
                process_command(&window, &gamepad_state, &gamepad_action_sender, &command);
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
    let clock_date_format = state.borrow().config.ui.clock.date_format.clone();
    let clock_format = state.borrow().config.ui.clock.format.clone();
    let clock_timer = Timer::default();
    clock_timer.start(TimerMode::Repeated, Duration::from_secs(30), move || {
        if let Some(window) = clock_window.upgrade() {
            window.set_clock_date(clock_text(&clock_date_format).into());
            window.set_clock_time(clock_text(&clock_format).into());
        }
    });

    window.window().set_fullscreen(true);
    window.run()?;
    Ok(())
}

fn wire_callbacks(window: &DeckWindow, state: Rc<RefCell<AppState>>, action_sender: mpsc::Sender<Result<(), String>>) {
    let weak_window = window.as_weak();
    let navigation_state = state.clone();
    let navigation_sender = action_sender.clone();
    window.on_navigate(move |command| {
        if let Some(window) = weak_window.upgrade() {
            process_command(&window, &navigation_state, &navigation_sender, command.as_str());
        }
    });

    let weak_window = window.as_weak();
    let view_navigation_state = state.clone();
    let view_navigation_sender = action_sender.clone();
    window.on_view_navigate(move |command| {
        if let Some(window) = weak_window.upgrade() {
            process_command(&window, &view_navigation_state, &view_navigation_sender, command.as_str());
        }
    });

    let weak_window = window.as_weak();
    let category_state = state.clone();
    window.on_focus_category(move |index| {
        if let Some(window) = weak_window.upgrade() {
            go_to_category(&window, &category_state, index as usize);
        }
    });

    let weak_window = window.as_weak();
    let pick_state = state.clone();
    let pick_sender = action_sender.clone();
    window.on_pick_item(move |index| {
        if let Some(window) = weak_window.upgrade() {
            pick_state.borrow_mut().position.pick(index as usize);
            activate_selected(&window, &pick_state, &pick_sender);
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
    let scroll_state = state.clone();
    window.on_scroll_items(move |delta| {
        if let Some(window) = weak_window.upgrade() {
            navigate(&window, &scroll_state, if delta < 0.0 { "down" } else { "up" });
        }
    });

    let weak_window = window.as_weak();
    window.on_back(move || {
        leave_example_view(weak_window.clone(), state.clone());
    });
}

fn leave_example_view(window: slint::Weak<DeckWindow>, state: Rc<RefCell<AppState>>) {
    Timer::single_shot(Duration::ZERO, move || {
        if let Some(window) = window.upgrade() {
            let mut state = state.borrow_mut();
            state.example_view = false;
            update_window(&window, &state);
        }
    });
}

fn apply_config(window: &DeckWindow, config: &config::Config) {
    let theme = window.global::<Theme>();
    theme.set_animation(config.ui.animation_ms as i64);
    theme.set_background(color(&config.ui.background, "#000000"));
    theme.set_font_family(config.ui.font_family.clone().into());
    theme.set_text(color(&config.ui.text, "#ffffff"));
    window.set_font_family(config.ui.font_family.clone().into());
    window.set_text_color(color(&config.ui.text, "#ffffff"));
    window.set_clock_date_color(color(&config.ui.clock.date_color, "#999999"));
    window.set_clock_date_font_percent(config.ui.clock.date_font_percent);
    window.set_clock_box_height_percent(config.ui.clock.box_height_percent);
    window.set_clock_box_width_percent(config.ui.clock.box_width_percent);
    window.set_clock_font_family(config.ui.clock.font_family.clone().into());
    window.set_clock_horizontal_alignment(config.ui.clock.horizontal_alignment.clone().into());
    window.set_clock_horizontal_padding_percent(config.ui.clock.horizontal_padding_percent);
    window.set_clock_line_spacing_percent(config.ui.clock.line_spacing_percent);
    window.set_clock_text_color(color(&config.ui.clock.text_color, "#ffffff"));
    window.set_clock_time_font_percent(config.ui.clock.time_font_percent);
    window.set_clock_top_offset_percent(config.ui.clock.top_offset_percent);
    window.set_clock_vertical_alignment(config.ui.clock.vertical_alignment.clone().into());
    window.set_header_category_font_percent(config.ui.header.category_font_percent);
    window.set_header_date_font_percent(config.ui.header.date_font_percent);
    window.set_header_date_top_percent(config.ui.header.date_top_percent);
    window.set_header_left_padding_percent(config.ui.header.left_padding_percent);
    window.set_header_time_font_percent(config.ui.header.time_font_percent);
    window.set_header_top_percent(config.ui.header.top_percent);
    window.set_example_back_button_font_percent(config.ui.example.back_button_font_percent);
    window.set_example_back_button_height_percent(config.ui.example.back_button_height_percent);
    window.set_example_back_button_left_percent(config.ui.example.back_button_left_percent);
    window.set_example_back_button_top_percent(config.ui.example.back_button_top_percent);
    window.set_example_back_button_width_percent(config.ui.example.back_button_width_percent);
    window.set_xmb_category_center_percent(config.ui.xmb.category_center_percent);
    window.set_xmb_category_center_y_percent(config.ui.xmb.category_center_y_percent);
    window.set_xmb_category_gap_percent(config.ui.xmb.category_gap_percent);
    window.set_xmb_category_icon_height_percent(config.ui.xmb.category_icon_height_percent);
    window.set_xmb_category_icon_percent(config.ui.xmb.category_icon_percent);
    window.set_xmb_category_icon_width_percent(config.ui.xmb.category_icon_width_percent);
    window.set_xmb_category_selected_icon_percent(config.ui.xmb.category_selected_icon_percent);
    window.set_xmb_error_bottom_percent(config.ui.xmb.error_bottom_percent);
    window.set_xmb_error_font_percent(config.ui.xmb.error_font_percent);
    window.set_xmb_error_horizontal_padding_percent(config.ui.xmb.error_horizontal_padding_percent);
    window.set_xmb_item_column_offset_percent(config.ui.xmb.item_column_offset_percent);
    window.set_xmb_item_column_width_percent(config.ui.xmb.item_column_width_percent);
    window.set_xmb_item_font_percent(config.ui.xmb.item_font_percent);
    window.set_xmb_item_glyph_offset_percent(config.ui.xmb.item_glyph_offset_percent);
    window.set_xmb_item_glyph_percent(config.ui.xmb.item_glyph_percent);
    window.set_xmb_item_glyph_width_percent(config.ui.xmb.item_glyph_width_percent);
    window.set_xmb_item_height_percent(config.ui.xmb.item_height_percent);
    window.set_xmb_item_label_offset_percent(config.ui.xmb.item_label_offset_percent);
    window.set_xmb_item_selected_font_percent(config.ui.xmb.item_selected_font_percent);
    window.set_xmb_item_selected_glyph_percent(config.ui.xmb.item_selected_glyph_percent);
    window.set_xmb_item_selected_height_percent(config.ui.xmb.item_selected_height_percent);
    window.set_xmb_item_selected_y_percent(config.ui.xmb.item_selected_y_percent);
    window.set_xmb_item_positive_near_step_percent(config.ui.xmb.item_positive_near_step_percent);
    window.set_xmb_item_positive_far_gap_percent(config.ui.xmb.item_positive_far_gap_percent);
    window.set_xmb_item_positive_far_step_percent(config.ui.xmb.item_positive_far_step_percent);
    window.set_xmb_item_negative_near_step_percent(config.ui.xmb.item_negative_near_step_percent);
    window.set_xmb_item_negative_far_top_percent(config.ui.xmb.item_negative_far_top_percent);
    window.set_xmb_item_negative_far_step_percent(config.ui.xmb.item_negative_far_step_percent);
    window.set_xmb_selection_line_percent(config.ui.xmb.selection_line_percent);
    window.set_xmb_selection_line_offset_percent(config.ui.xmb.selection_line_offset_percent);
    window.set_xmb_vertical_offset_percent(config.ui.xmb.vertical_offset_percent);
    window.set_xmb_viewport_top_percent(config.ui.xmb.viewport_top_percent);
    if let Ok(image) = slint::Image::load_from_path(std::path::Path::new(&config.ui.background_image)) {
        theme.set_background_image(image);
        theme.set_has_background_image(true);
    }
}

fn clock_text(format: &str) -> String {
    std::process::Command::new("date")
        .arg(format!("+{format}"))
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
    let cells = state
        .root
        .iter()
        .enumerate()
        .map(|(index, item)| Cell {
            glyph: item.glyph.as_str().into(),
            index: index as i32,
            label: item.label.as_str().into(),
        })
        .collect::<Vec<_>>();
    if state.cells.row_count() == cells.len() {
        for (row, cell) in cells.into_iter().enumerate() {
            state.cells.set_row_data(row, cell);
        }
    } else {
        state.cells.set_vec(cells);
    }
    window.set_busy(state.busy);
    window.set_example_view(state.example_view);
    window.set_error_message(state.error_message.clone().unwrap_or_default().into());
    window.set_category_title(state.categories[state.position.category].clone().into());
    window.set_selected_category(state.position.category as i32);
    window.set_selected_item(state.position.item as i32);
}

fn go_to_category(window: &DeckWindow, state: &Rc<RefCell<AppState>>, category: usize) {
    let mut state = state.borrow_mut();
    let category = category.min(state.categories.len() - 1);
    state.root = menu::build(&state.config, &state.categories[category]);
    state.position.go_to_category(category);
    update_window(window, &state);
}

fn navigate(window: &DeckWindow, state: &Rc<RefCell<AppState>>, command: &str) {
    let (category_count, row_count, home, current) = {
        let state = state.borrow();
        let home = state.categories.iter().position(|name| name == "Home").unwrap_or(0);
        (state.categories.len(), state.root.len(), home, state.position.category)
    };
    match command {
        "left" => {
            let mut target = navigation::Position::new(current);
            target.move_category(category_count, -1);
            if target.category != current {
                go_to_category(window, state, target.category);
            }
        }
        "right" => {
            if current + 1 < category_count {
                let mut target = navigation::Position::new(current);
                target.move_category(category_count, 1);
                go_to_category(window, state, target.category);
            }
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
    let action = {
        let state = state.borrow();
        state.root.get(state.position.item).map(|item| item.action.clone())
    };
    match action {
        Some(MenuAction::ExampleView) => {
            let mut state = state.borrow_mut();
            state.example_view = true;
            update_window(window, &state);
        }
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

fn process_command(
    window: &DeckWindow,
    state: &Rc<RefCell<AppState>>,
    action_sender: &mpsc::Sender<Result<(), String>>,
    command: &str,
) {
    if state.borrow().example_view {
        if matches!(command, "back" | "left" | "menu" | "select") {
            leave_example_view(window.as_weak(), state.clone());
        }
        return;
    }
    if command == "select" {
        activate_selected(window, state, action_sender);
    } else {
        navigate(window, state, command);
    }
}

fn show_error(window: &DeckWindow, state: &Rc<RefCell<AppState>>, error: String) {
    let mut state = state.borrow_mut();
    state.error_message = Some(error);
    update_window(window, &state);
}
