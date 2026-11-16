use crate::deck::{deck, message, navigation};
use crate::menu::{bluetooth_glyph, category_glyph, icon_check, item, network_glyph, volume_glyph};
use iced::alignment::Vertical;
use iced::mouse::{Button, ScrollDelta};
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, Text};
use iced::widget::{button, column, container, image, row, stack, text, text::Alignment};
use iced::{Color, ContentFit, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

pub const background: Color = Color::BLACK;
const lift: f32 = 0.3;
const main_big: f32 = 0.6;
const main_small: f32 = 0.5;
const dim: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
pub const foreground: Color = Color::WHITE;
const subdued: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.6);

// use the configured image as a backdrop, with the canvas drawing the interface.
pub fn view(deck: &deck) -> Element<'_, message> {
    let canvas = canvas::Canvas::new(deck).width(Length::Fill).height(Length::Fill);
    let view: Element<'_, message> = match deck.config.ui.background.as_str() {
        "" => canvas.into(),
        path => {
            stack![image(path).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill), canvas].into()
        }
    };
    if let Some(prompt) = &deck.wifi_prompt { stack![view, wifi_prompt_view(prompt)].into() } else { view }
}

fn wifi_prompt_view(prompt: &crate::deck::wifi_prompt) -> Element<'_, message> {
    let keys: Vec<char> = crate::deck::wifi_keys.chars().collect();
    let mut keyboard = column![].spacing(8);
    for (row_index, row_keys) in keys.chunks(12).enumerate() {
        let mut key_row = row![].spacing(8).width(Length::Fill);
        for (column_index, key) in row_keys.iter().enumerate() {
            let index = row_index * 12 + column_index;
            let key = if prompt.uppercase { key.to_ascii_uppercase() } else { *key };
            key_row = key_row.push(
                button(text(key.to_string()).size(24))
                    .width(Length::Fill)
                    .padding(12)
                    .style(move |_, status| key_button_style(index == prompt.selected_key, status))
                    .on_press_maybe(prompt.requested.then_some(message::wifi_character(key))),
            );
        }
        keyboard = keyboard.push(key_row);
    }
    let shift = button(if prompt.uppercase { "ABC" } else { "abc" })
        .on_press_maybe(prompt.requested.then_some(message::wifi_shift));
    keyboard = keyboard.push(shift);
    let actions = row![
        button("delete").on_press_maybe(prompt.requested.then_some(message::wifi_delete)),
        button("clear").on_press_maybe(prompt.requested.then_some(message::wifi_clear)),
        button("cancel").on_press_maybe(prompt.requested.then_some(message::wifi_cancel)),
        button(if crate::deck::valid_wifi_passphrase(&prompt.passphrase) { "connect" } else { "8+ characters" })
            .on_press_maybe(
                (prompt.requested && crate::deck::valid_wifi_passphrase(&prompt.passphrase))
                    .then_some(message::wifi_submit),
            ),
    ]
    .spacing(16);
    let panel = column![
        text(format!("Connect to {}", prompt.network)).size(32),
        text(if prompt.requested { "Enter the Wi-Fi passphrase" } else { "Waiting for iwd…" }).size(20),
        container(text("• ".repeat(prompt.passphrase.chars().count())).size(26)).padding(16).width(Length::Fill).style(
            |_| container::Style {
                background: Some(Color::from_rgb(0.08, 0.08, 0.08).into()),
                border: iced::Border { color: subdued, width: 1.0, radius: 4.0.into() },
                ..Default::default()
            }
        ),
        keyboard,
        actions,
    ]
    .spacing(18)
    .width(Length::Fixed(1100.0));
    container(container(panel).width(Length::Fill).height(Length::Fill).center(Length::Fill).style(|_| {
        container::Style { background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.8).into()), ..Default::default() }
    }))
    .into()
}

fn key_button_style(selected: bool, status: iced::widget::button::Status) -> iced::widget::button::Style {
    let fill = if selected || status == iced::widget::button::Status::Hovered {
        foreground
    } else {
        Color::from_rgb(0.15, 0.15, 0.15)
    };
    iced::widget::button::Style {
        background: Some(fill.into()),
        border: iced::Border { color: foreground, width: 1.0, radius: 4.0.into() },
        text_color: if selected || status == iced::widget::button::Status::Hovered { fill } else { foreground },
        ..Default::default()
    }
}

// anchor menu columns slightly left of the screen center, sliding them with the current depth.
fn column_x(unit: f32, width: f32, column: usize, depth: f32) -> f32 {
    width / 2.0 - unit + (column as f32 - depth) * unit * 3.5
}

// place and scale a row so nearby choices form the focused menu layout, shifted up to sit below the header.
fn spot(unit: f32, column: usize, index: usize, selected: f32) -> (f32, f32) {
    let offset = index as f32 - selected;
    let (big, small) = if column > 0 { (0.35, 0.25) } else { (0.45, 0.3) };
    let y = match offset {
        _ if offset.abs() < 1.0 && (offset > 0.0 || column > 0) => 4.6 + offset * 0.75,
        _ if offset > 0.0 || column > 0 => 4.6 + offset.signum() * 0.25 + offset * 0.5,
        _ if offset > -1.0 => 4.6 + offset * 2.0,
        _ => 2.6 + (offset + 1.0) * 0.5,
    };
    (unit * (y - lift), unit * (small + (big - small) * (1.0 - offset.abs()).max(0.0)))
}

// rows above the main menu that would climb into the header are hidden.
fn visible(unit: f32, y: f32) -> bool {
    y >= unit * 1.5
}

// measure a label so its selection underline matches the rendered text width.
fn measure(content: &str, size: f32) -> f32 {
    use iced::advanced::text::{self, Paragraph as _};
    let text = text::Text {
        align_x: Alignment::Left,
        align_y: Vertical::Top,
        bounds: Size::INFINITE,
        content,
        font: Font::DEFAULT,
        line_height: text::LineHeight::default(),
        shaping: text::Shaping::default(),
        size: size.into(),
        wrapping: text::Wrapping::None,
    };
    <Renderer as text::Renderer>::Paragraph::with_text(text).min_width()
}

// handle pointer interaction and draw the complete interface using iced's canvas.
impl canvas::Program<message> for deck {
    type State = ();

    // convert pointer events into the same pick and navigation messages as other input.
    fn update(&self, _: &mut (), event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<message>> {
        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            return Some(Action::publish(message::mouse));
        }
        let point = cursor.position_in(bounds)?;
        let unit = bounds.height / 10.0;
        let display_position = self.get_display_position();
        let message = match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                let columns = self.get_selection_category_breadcrumb(self.get_category_items());
                let depth = columns.len() - 1;
                let category = (0..crate::config::get_configured_categories(&self.config).len())
                    .filter(|_| depth < 2)
                    .find(|index| {
                        let x = column_x(unit, bounds.width, 0, display_position[1])
                            + (*index as f32 - display_position[0]) * unit * 1.4;
                        (point.x - x).abs() < unit * 0.5 && (point.y - unit * (3.6 - lift)).abs() < unit * 0.4
                    });
                let row = columns.iter().enumerate().skip(depth.saturating_sub(1)).find_map(|(column, items)| {
                    let x = column_x(unit, bounds.width, column, display_position[1]);
                    (0..items.len()).find_map(|index| {
                        let (y, _) =
                            spot(unit, column, index, display_position.get(column + 2).copied().unwrap_or(0.0));
                        let hit = visible(unit, y)
                            && point.x > x - unit * 0.5
                            && point.x < x + unit * 5.0
                            && (point.y - y).abs() < unit * 0.25;
                        hit.then(|| [&self.selected_rows[..column], &[index]].concat())
                    })
                });
                match (category, row) {
                    (Some(category_index), _) => message::pick { category_index, selected_rows: Vec::new() },
                    (_, Some(selected_rows)) => message::pick { category_index: self.category, selected_rows },
                    _ => return None,
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(Button::Right)) => message::nav(navigation::back),
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let (ScrollDelta::Lines { x, y } | ScrollDelta::Pixels { x, y }) = *delta;
                message::nav(match (x.abs() > y.abs(), x > 0.0, y > 0.0) {
                    (true, true, _) => navigation::left,
                    (true, false, _) => navigation::right,
                    (false, _, true) => navigation::up,
                    (false, _, false) => navigation::down,
                })
            }
            _ => return None,
        };
        Some(Action::publish(message).and_capture())
    }

    // hide the pointer after it has been idle for a short time.
    fn mouse_interaction(&self, _: &(), _: Rectangle, _: mouse::Cursor) -> mouse::Interaction {
        let active =
            self.last_pointer_activity.is_some_and(|last_activity| last_activity.elapsed().as_secs_f32() < 4.0);
        if active { mouse::Interaction::default() } else { mouse::Interaction::Hidden }
    }

    // draw status, page navigation, nested menu rows, and current-level indicators.
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut pen = pen { frame: Frame::new(renderer, bounds.size()), wide: bounds.height * 0.48 };
        let (unit, right) = (bounds.height / 10.0, bounds.width - bounds.height * 0.06);
        let layout = layout { header: bounds.height / 4.0, height: bounds.height, right, unit, width: bounds.width };
        let columns = self.get_selection_category_breadcrumb(self.get_category_items());
        self.draw_header(&mut pen, &layout);
        self.draw_footer(&mut pen, &layout);
        self.draw_main_menu(&mut pen, &layout, &columns);
        pen.wide = unit * 4.0;
        let lines = self.draw_submenu(&mut pen, &layout, &columns);
        draw_underlines(&mut pen, &layout, &lines);
        vec![pen.frame.into_geometry()]
    }
}

struct layout {
    header: f32,
    height: f32,
    right: f32,
    unit: f32,
    width: f32,
}

impl layout {
    fn date_gap(&self) -> f32 {
        self.unit * 0.3
    }

    fn header_top(&self) -> f32 {
        self.unit * 0.3
    }

    fn time_size(&self) -> f32 {
        self.header * 0.45
    }

    fn time_bearing(&self) -> f32 {
        self.time_size() * 0.08
    }

    fn title_size(&self) -> f32 {
        self.header * 0.26
    }
}

struct pen {
    frame: Frame,
    wide: f32,
}

impl pen {
    fn put(&mut self, content: &str, at: (f32, f32), size: f32, color: Color, align: Alignment, font: Font) {
        let vertical = if align == Alignment::Left { Vertical::Top } else { Vertical::Center };
        self.put_aligned(content, at, size, color, (align, vertical), font);
    }

    fn put_aligned(
        &mut self,
        content: &str,
        at: (f32, f32),
        size: f32,
        color: Color,
        (align, vertical): (Alignment, Vertical),
        font: Font,
    ) {
        let left = align == Alignment::Left;
        self.frame.fill_text(Text {
            align_x: align,
            align_y: vertical,
            color,
            content: content.into(),
            font,
            max_width: if left { self.wide } else { f32::INFINITY },
            position: Point::new(at.0, at.1),
            size: size.into(),
            ..Text::default()
        });
    }
}

impl deck {
    // draw the header: title, time, and date label.
    fn draw_header(&self, pen: &mut pen, layout: &layout) {
        self.draw_title(pen, layout);
        self.draw_time(pen, layout);
        self.draw_date(pen, layout);
    }

    // draw the main category name on the left, limited to the space before the time.
    fn draw_title(&self, pen: &mut pen, layout: &layout) {
        let categories = crate::config::get_configured_categories(&self.config);
        let title = categories.get(self.category).map_or("", String::as_str);
        let time_width = measure(&self.system_snapshot.time, layout.time_size());
        pen.wide = (layout.right - time_width - layout.time_bearing() - layout.unit * 1.2).max(layout.unit);
        let at = (layout.unit * 0.6, layout.header_top());
        let center = (Alignment::Left, Vertical::Top);
        pen.put_aligned(title, at, layout.title_size(), foreground, center, Font::DEFAULT);
    }

    // draw the clock on the right of the header.
    fn draw_time(&self, pen: &mut pen, layout: &layout) {
        let center = (Alignment::Right, Vertical::Top);
        let at = (layout.right + layout.time_bearing(), layout.header_top());
        pen.put_aligned(&self.system_snapshot.time, at, layout.time_size(), foreground, center, Font::DEFAULT);
    }

    // draw the small date label below the time.
    fn draw_date(&self, pen: &mut pen, layout: &layout) {
        let at = (layout.right, layout.header_top() + layout.time_size() + layout.date_gap());
        pen.put(&self.system_snapshot.date, at, layout.unit * 0.18, subdued, Alignment::Right, Font::DEFAULT);
    }

    // draw the notice, connectivity glyphs, volume bar, and host metrics in the bottom bar.
    fn draw_footer(&self, pen: &mut pen, layout: &layout) {
        let (unit, right, snapshot) = (layout.unit, layout.right, &self.system_snapshot);
        let (icon, system, y) = (self.font_icons, &snapshot.host_metrics, layout.height - unit * 0.35);
        pen.wide = unit * 4.8;
        let notice = self.busy_label.as_ref().map(|busy| format!("{busy}…")).or_else(|| self.error_text.clone());
        let center = (Alignment::Left, Vertical::Center);
        pen.put_aligned(&notice.unwrap_or_default(), (unit * 0.6, y), unit * 0.15, dim, center, Font::DEFAULT);
        let temperature = system.temperature.map_or(String::new(), |value| format!("   \u{f050f} {value}°C"));
        let stats = format!(
            "\u{f0ee0} {}%{temperature}   \u{f035b} {}   \u{f02ca} {}   \u{f0453} {} ms",
            system.cpu, system.memory, system.disk, system.ping
        );
        pen.put(&stats, (right, y), unit * 0.1, subdued, Alignment::Right, icon);
        let mut x = right - measure(&stats, unit * 0.1) - unit * 0.5;
        let glyphs =
            format!("{}   {}   {}", network_glyph(snapshot), bluetooth_glyph(snapshot), volume_glyph(snapshot));
        pen.put(&glyphs, (x, y), unit * 0.12, subdued, Alignment::Right, icon);
        let Some(audio) = snapshot.audio.as_ref() else { return };
        x -= measure(&glyphs, unit * 0.12) + unit * 0.2;
        let (bar, level) = (unit * 1.2, if audio.is_muted { 0 } else { audio.volume.min(100) });
        let at = Point::new(x - unit * 1.2, y - unit * 0.01);
        pen.frame.fill_rectangle(at, Size::new(bar, unit * 0.02), dim);
        pen.frame.fill_rectangle(at, Size::new(bar * level as f32 / 100.0, unit * 0.02), foreground);
    }

    // draw the main menu: the horizontal category icons below the title.
    fn draw_main_menu(&self, pen: &mut pen, layout: &layout, columns: &[&[item]]) {
        let (unit, depth) = (layout.unit, columns.len() - 1);
        let categories = crate::config::get_configured_categories(&self.config);
        let display_position = self.get_display_position();
        for (index, page) in categories.iter().enumerate().filter(|_| depth < 2) {
            let x = column_x(unit, layout.width, 0, display_position[1])
                + (index as f32 - display_position[0]) * unit * 1.4;
            let focused = index == self.category;
            let color = match (focused, depth) {
                (true, 0) => foreground,
                (true, _) => subdued,
                _ => dim,
            };
            let size = unit * if focused { main_big } else { main_small };
            let glyph = category_glyph(&self.config, page);
            pen.put(glyph, (x, unit * (3.6 - lift)), size, color, Alignment::Center, self.font_icons);
        }
    }

    // draw the submenu rows and return the selection underlines to draw on top.
    fn draw_submenu(&self, pen: &mut pen, layout: &layout, columns: &[&[item]]) -> Vec<(f32, f32, f32)> {
        let (unit, depth, icon) = (layout.unit, columns.len() - 1, self.font_icons);
        let display_position = self.get_display_position();
        let mut lines = Vec::new();
        for (column, items) in columns.iter().enumerate().skip(depth.saturating_sub(1)) {
            let x = column_x(unit, layout.width, column, display_position[1]);
            let selected = self.selected_rows[column].min(items.len().saturating_sub(1));
            for (index, item) in items.iter().enumerate() {
                let focus = display_position.get(column + 2).copied().unwrap_or(selected as f32);
                let (y, size) = spot(unit, column, index, focus);
                if !visible(unit, y) {
                    continue;
                }
                let color = match (index == selected, column == depth) {
                    (true, true) => foreground,
                    (true, false) => subdued,
                    _ => dim,
                };
                let checked = item.value == icon_check;
                let glyph_size = (size * 0.7).min(unit * main_small);
                pen.put(&item.glyph, (x, y), glyph_size, color, Alignment::Center, icon);
                let mark = if checked { icon_check } else { "" };
                pen.put(mark, (x - unit * 0.5, y), unit * 0.18, color, Alignment::Center, icon);
                let at = (x + unit * 0.5, y - size * 0.5);
                pen.put(&item.label, at, size * 0.5, color, Alignment::Left, Font::DEFAULT);
                if index == selected && column == depth {
                    lines.push((x + unit * 0.5, y + size * 0.08, measure(&item.label, size * 0.5).min(unit * 4.0)));
                }
                if !checked && index == selected {
                    let at = (x + unit * 0.5, y + size * 0.15);
                    pen.put(&item.value, at, size * 0.35, subdued, Alignment::Left, Font::DEFAULT);
                }
            }
        }
        lines
    }
}

// draw the underline beneath each focused label.
fn draw_underlines(pen: &mut pen, layout: &layout, lines: &[(f32, f32, f32)]) {
    for &(x, y, width) in lines {
        pen.frame.fill_rectangle(Point::new(x, y), Size::new(width, layout.unit * 0.015), foreground);
    }
}
