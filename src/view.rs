use crate::deck::{deck, message, navigation};
use crate::menu::{bluetooth_glyph, category_glyph, icon_check, network_glyph, volume_glyph};
use iced::alignment::Vertical;
use iced::mouse::{Button, ScrollDelta};
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, Text};
use iced::widget::{image, stack, text::Alignment};
use iced::{Color, ContentFit, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

pub const background: Color = Color::BLACK;
const dim: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
pub const foreground: Color = Color::WHITE;
const subdued: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.6);

// use the configured image as a backdrop, with the canvas drawing the interface.
pub fn view(deck: &deck) -> Element<'_, message> {
    let canvas = canvas::Canvas::new(deck).width(Length::Fill).height(Length::Fill);
    match deck.config.ui.background.as_str() {
        "" => canvas.into(),
        path => {
            stack![image(path).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill), canvas].into()
        }
    }
}

// position each menu column relative to the center and current depth.
fn column_x(unit: f32, width: f32, column: usize, depth: f32) -> f32 {
    width / 2.0 - unit + (column as f32 - depth) * unit * 3.5
}

// place and scale a row so nearby choices form the focused menu layout.
fn spot(unit: f32, column: usize, index: usize, selected: f32) -> (f32, f32) {
    let offset = index as f32 - selected;
    let (big, small) = if column > 0 { (0.35, 0.25) } else { (0.45, 0.3) };
    let y = match offset {
        _ if offset.abs() < 1.0 && (offset > 0.0 || column > 0) => 4.6 + offset * 0.75,
        _ if offset > 0.0 || column > 0 => 4.6 + offset.signum() * 0.25 + offset * 0.5,
        _ if offset > -1.0 => 4.6 + offset * 2.0,
        _ => 2.6 + (offset + 1.0) * 0.5,
    };
    (unit * y, unit * (small + (big - small) * (1.0 - offset.abs()).max(0.0)))
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
                        (point.x - x).abs() < unit * 0.5 && (point.y - unit * 3.6).abs() < unit * 0.4
                    });
                let row = columns.iter().enumerate().skip(depth.saturating_sub(1)).find_map(|(column, items)| {
                    let x = column_x(unit, bounds.width, column, display_position[1]);
                    (0..items.len()).find_map(|index| {
                        let (y, _) =
                            spot(unit, column, index, display_position.get(column + 2).copied().unwrap_or(0.0));
                        let hit =
                            point.x > x - unit * 0.5 && point.x < x + unit * 5.0 && (point.y - y).abs() < unit * 0.25;
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
        let mut frame = Frame::new(renderer, bounds.size());
        let (width, unit) = (bounds.width, bounds.height / 10.0);
        let (icon, label, snapshot, right) =
            (self.font_icons, Font::DEFAULT, &self.system_snapshot, width - unit * 0.6);
        let wide = std::cell::Cell::new(unit * 4.8);
        let mut put = |content: &str, at: (f32, f32), size: f32, color: Color, align: Alignment, font: Font| {
            let max_width = if align == Alignment::Left { wide.get() } else { f32::INFINITY };
            let align_y = if align == Alignment::Left { Vertical::Top } else { Vertical::Center };
            let (content, position, size) = (content.into(), Point::new(at.0, at.1), size.into());
            frame.fill_text(Text {
                align_x: align,
                align_y,
                color,
                content,
                font,
                max_width,
                position,
                size,
                ..Text::default()
            });
        };
        let system = &snapshot.host_metrics;
        let temperature = system.temperature.map_or(String::new(), |value| format!("   \u{f050f} {value}°C"));
        let stats = format!(
            "\u{f0ee0} {}%{temperature}   \u{f035b} {}   \u{f02ca} {}   \u{f0453} {} ms",
            system.cpu, system.memory, system.disk, system.ping
        );
        put(&snapshot.time, (right, unit * 1.05), unit * 0.7, foreground, Alignment::Right, label);
        put(&snapshot.date, (right, unit * 1.6), unit * 0.18, subdued, Alignment::Right, label);
        put(&stats, (right, unit * 1.95), unit * 0.1, subdued, Alignment::Right, icon);
        let bar = unit * 1.2;
        let glyphs =
            format!("{}   {}   {}", network_glyph(snapshot), bluetooth_glyph(snapshot), volume_glyph(snapshot));
        let x = if snapshot.audio.is_some() { right - bar - unit * 0.12 } else { right };
        put(&glyphs, (x, unit * 2.35), unit * 0.12, subdued, Alignment::Right, icon);
        let notice = self.busy_label.as_ref().map(|busy| format!("{busy}…")).or_else(|| self.error_text.clone());
        put(
            &notice.unwrap_or_default(),
            (unit * 0.6, bounds.height - unit * 0.8),
            unit * 0.2,
            dim,
            Alignment::Left,
            label,
        );
        let columns = self.get_selection_category_breadcrumb(self.get_category_items());
        let depth = columns.len() - 1;
        let categories = crate::config::get_configured_categories(&self.config);
        let page = categories.get(self.category).map_or("", String::as_str);
        put(page, (unit * 0.6, unit * 0.5), unit * 0.3, foreground, Alignment::Left, label);
        let display_position = self.get_display_position();
        for (index, page) in categories.iter().enumerate().filter(|_| depth < 2) {
            let x = column_x(unit, bounds.width, 0, display_position[1])
                + (index as f32 - display_position[0]) * unit * 1.4;
            let focused = index == self.category;
            let color = if focused && depth == 0 {
                foreground
            } else if focused {
                subdued
            } else {
                dim
            };
            let size = unit * if focused { 0.6 } else { 0.5 };
            put(category_glyph(&self.config, page), (x, unit * 3.6), size, color, Alignment::Center, icon);
        }
        wide.set(unit * 4.0);
        let mut lines = Vec::new();
        for (column, items) in columns.iter().enumerate().skip(depth.saturating_sub(1)) {
            let (x, selected) = (
                column_x(unit, bounds.width, column, display_position[1]),
                self.selected_rows[column].min(items.len().saturating_sub(1)),
            );
            for (index, item) in items.iter().enumerate() {
                let (y, size) =
                    spot(unit, column, index, display_position.get(column + 2).copied().unwrap_or(selected as f32));
                let color = match (index == selected, column == depth) {
                    (true, true) => foreground,
                    (true, false) => subdued,
                    _ => dim,
                };
                let checked = item.value == icon_check;
                put(&item.glyph, (x, y), size, color, Alignment::Center, icon);
                put(
                    if checked { icon_check } else { "" },
                    (x - unit * 0.5, y),
                    unit * 0.18,
                    color,
                    Alignment::Center,
                    icon,
                );
                put(&item.label, (x + unit * 0.5, y - size * 0.5), size * 0.5, color, Alignment::Left, label);
                if index == selected && column == depth {
                    lines.push((x + unit * 0.5, y + size * 0.08, measure(&item.label, size * 0.5).min(unit * 4.0)));
                }
                if !checked && index == selected {
                    put(&item.value, (x + unit * 0.5, y + size * 0.15), size * 0.35, subdued, Alignment::Left, label);
                }
            }
        }
        let level = snapshot.audio.as_ref().map(|audio| if audio.is_muted { 0 } else { audio.volume.min(100) });
        if let Some(level) = level {
            frame.fill_rectangle(Point::new(right - bar, unit * 2.34), Size::new(bar, unit * 0.02), dim);
            let size = Size::new(bar * level as f32 / 100.0, unit * 0.02);
            frame.fill_rectangle(Point::new(right - bar, unit * 2.34), size, foreground);
        }
        for (x, y, width) in lines {
            frame.fill_rectangle(Point::new(x, y), Size::new(width, unit * 0.015), foreground);
        }
        vec![frame.into_geometry()]
    }
}
