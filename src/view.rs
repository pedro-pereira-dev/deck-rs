use crate::deck::{deck, message, nav, panel};
use crate::items::{bluetooth_glyph, network_glyph, volume_glyph};
use iced::widget::{button, center, column, container, mouse_area, opaque, row, space, stack, text};
use iced::{Alignment, Border, Color, Element, Length, mouse};
use std::time::Duration;

pub const accent: Color = Color::from_rgb8(0x8a, 0xb4, 0xff);
pub const background: Color = Color::from_rgb8(0x00, 0x00, 0x00);
const dim: Color = Color::from_rgb8(0x6b, 0x6b, 0x78);
pub const foreground: Color = Color::from_rgb8(0xec, 0xec, 0xf1);
const raised: Color = Color::from_rgb8(0x23, 0x23, 0x2c);
const scrim: Color = Color::from_rgba8(0x00, 0x00, 0x00, 0.6);
const surface: Color = Color::from_rgb8(0x17, 0x17, 0x1e);

pub fn view(deck: &deck) -> Element<'_, message> {
    let status = &deck.status;
    let focused = |cell| deck.panel.is_none() && deck.focus == cell;
    let indicators = row![
        text(network_glyph(status)).size(22),
        text(bluetooth_glyph(status)).size(22),
        text(volume_glyph(status)).size(22),
        text!("{}%", status.volume).color(dim).size(16),
    ]
    .align_y(Alignment::Center)
    .spacing(20);
    let top = row![text(&status.clock).size(28), space::horizontal(), indicators].align_y(Alignment::Center);
    let tiles = row(deck.apps.iter().enumerate().map(|(index, app)| {
        let size = if focused((0, index)) { 232 } else { 200 };
        let content =
            column![text(&app.icon).size(72), text(&app.name).size(18)].align_x(Alignment::Center).spacing(16);
        button(center(content))
            .height(size)
            .on_press(message::press(0, index))
            .padding(24)
            .style(tile(focused((0, index)), 28.0))
            .width(size)
            .into()
    }))
    .align_y(Alignment::Center)
    .height(260)
    .spacing(32);
    let pills = row(panel::all.iter().enumerate().map(|(index, panel)| {
        let content = row![text(panel.glyph(status)).size(20), text!("{panel:?}").size(16)]
            .align_y(Alignment::Center)
            .spacing(12);
        button(content)
            .on_press(message::press(1, index))
            .padding([14, 26])
            .style(tile(focused((1, index)), 999.0))
            .into()
    }))
    .spacing(20);
    let ping = match deck.ping {
        Some(ping) => format!("ping {ping:.0} ms   {}", status.address),
        None => "not connected".into(),
    };
    let footer = row![
        text!("{}   {}   {}   {ping}", deck.cpu, deck.memory, deck.disk).color(dim).size(14),
        space::horizontal(),
        text(&deck.updates).color(accent).size(14),
        space::horizontal(),
        text(&deck.info).color(dim).size(14),
    ]
    .spacing(24);
    let home = column![
        top,
        space::vertical(),
        center(tiles).height(Length::Shrink),
        center(pills).height(Length::Shrink),
        space::vertical(),
        footer
    ]
    .spacing(48);
    let mut layers = stack![container(home).height(Length::Fill).padding([40, 64]).width(Length::Fill)];
    if let Some(panel) = deck.panel {
        layers = layers.push(overlay(deck, panel));
    }
    if !deck.cursor {
        let blank = mouse_area(space().height(Length::Fill).width(Length::Fill));
        layers = layers.push(blank.interaction(mouse::Interaction::Hidden));
    }
    layers.into()
}

fn overlay(deck: &deck, panel: panel) -> Element<'_, message> {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let title = match (&deck.busy, &deck.error) {
        (Some(busy), _) => format!("{panel:?}   {} {busy}…", frames[deck.spinner % frames.len()]),
        (None, Some((error, at))) if at.elapsed() < Duration::from_secs(6) => format!("{panel:?}   {error}"),
        _ => format!("{panel:?}"),
    };
    let entries = deck.items.iter().enumerate().map(|(index, item)| {
        let arrows = if item.left.is_some() { "\u{f0141}  \u{f0142}" } else { "" };
        let content = row![
            text(&item.glyph).size(20).width(36),
            text(&item.label).size(17),
            space::horizontal(),
            text(&item.value).color(dim).size(15),
            text(arrows).color(dim).size(15),
        ]
        .align_y(Alignment::Center)
        .spacing(12);
        button(content)
            .on_press(message::press(0, index))
            .padding([14, 20])
            .style(tile(deck.index == index, 14.0))
            .width(Length::Fill)
            .into()
    });
    let card = container(column![text(title).size(26), column(entries).spacing(6)].spacing(24))
        .padding(32)
        .style(|_| container::Style {
            background: Some(surface.into()),
            border: Border::default().rounded(28),
            ..Default::default()
        })
        .width(560);
    let overlay = center(card).style(|_| container::Style { background: Some(scrim.into()), ..Default::default() });
    opaque(mouse_area(overlay).on_press(message::nav(nav::back)))
}

fn tile(focused: bool, radius: f32) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, state| {
        let hovered = matches!(state, button::Status::Hovered | button::Status::Pressed);
        let fill = match (focused, hovered) {
            (true, _) => raised,
            (false, true) => surface,
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(fill.into()),
            border: Border::default().color(if focused { accent } else { Color::TRANSPARENT }).rounded(radius).width(2),
            text_color: Color { a: if focused { 1.0 } else { 0.7 }, ..foreground },
            ..Default::default()
        }
    }
}
