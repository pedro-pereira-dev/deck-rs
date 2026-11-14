use crate::deck::{deck, message};
use crate::items::{bluetooth_glyph, network_glyph, title, volume_glyph};
use iced::widget::{button, column, container, row, space, text};
use iced::{Alignment, Border, Color, Element, Length};

pub const background: Color = Color::from_rgb8(0x10, 0x12, 0x18);
const dim: Color = Color::from_rgb8(0x80, 0x84, 0x90);
pub const foreground: Color = Color::from_rgb8(0xf0, 0xf0, 0xf4);
const highlight: Color = Color::from_rgb8(0x2a, 0x30, 0x3c);

pub fn view(deck: &deck) -> Element<'_, message> {
    let status = &deck.status;
    let notice = match (&deck.busy, &deck.error) {
        (Some(busy), _) => format!("{busy}…"),
        (None, Some(error)) => error.clone(),
        _ => String::new(),
    };
    let icon = |glyph| text(glyph).font(deck.icon).size(24);
    let mut indicators = row![icon(network_glyph(status)), icon(bluetooth_glyph(status))];
    if let Some(audio) = &status.audio {
        indicators = indicators.push(icon(volume_glyph(status))).push(text!("{}%", audio.volume).size(20));
    }
    let header = row![
        text(notice).color(dim).size(20),
        space::horizontal(),
        indicators.spacing(20).align_y(Alignment::Center),
        text(&status.clock).size(24),
    ]
    .align_y(Alignment::Center)
    .spacing(32);
    let pages = deck.pages();
    let menu = column(pages.iter().enumerate().map(|(index, page)| {
        let (glyph, label) = title(deck, *page);
        let focused = index == deck.category;
        let size = if focused { 44 } else { 28 };
        let color = if focused && !deck.inside { foreground } else { dim };
        let content = row![text(glyph).font(deck.icon).size(size).width(64), text(label).size(size)]
            .align_y(Alignment::Center)
            .spacing(16);
        button(content).on_press(message::press(false, index)).padding([8, 0]).style(style(false, color)).into()
    }));
    let (glyph, label) = title(deck, pages[deck.category]);
    let items = deck.items();
    let empty = items.is_empty().then(|| text("unavailable").color(dim).size(20));
    let rows = column(items.into_iter().enumerate().map(|(index, item)| {
        let arrows = if item.left.is_some() { "\u{f0141}  \u{f0142}" } else { "" };
        let content = row![
            text(item.glyph).font(deck.icon).size(24).width(40),
            text(item.label).size(22),
            space::horizontal(),
            text(item.value).color(dim).size(20),
            text(arrows).font(deck.icon).color(dim).size(20),
        ]
        .align_y(Alignment::Center)
        .spacing(16);
        let focused = deck.inside && index == deck.row;
        let color = if deck.inside { foreground } else { dim };
        button(content)
            .on_press(message::press(true, index))
            .padding([16, 24])
            .style(style(focused, color))
            .width(Length::Fill)
            .into()
    }))
    .push(empty)
    .spacing(4);
    let pane = column![
        row![text(glyph).font(deck.icon).size(32), text(label).size(32)].align_y(Alignment::Center).spacing(16),
        rows,
    ]
    .spacing(32);
    let body = row![
        container(menu).center_y(Length::Fill).width(Length::FillPortion(2)),
        container(pane).center_y(Length::Fill).width(Length::FillPortion(3)),
    ]
    .spacing(96);
    column![header, body].padding([56, 96]).spacing(32).into()
}

fn style(focused: bool, color: Color) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: focused.then_some(highlight.into()),
        border: Border::default().rounded(4),
        text_color: color,
        ..Default::default()
    }
}
