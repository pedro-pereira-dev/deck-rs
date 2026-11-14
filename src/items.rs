use crate::deck::{action, deck, message, page};
use crate::modules::bluetooth::state;
use crate::status::{link, status};

#[derive(Clone)]
pub struct item {
    pub glyph: String,
    pub label: String,
    pub left: Option<message>,
    pub right: Option<message>,
    pub select: Option<message>,
    pub value: String,
}

pub fn entry(glyph: &str, label: &str, value: &str, select: Option<message>) -> item {
    item { glyph: glyph.into(), label: label.into(), left: None, right: None, select, value: value.into() }
}

fn run(label: &str, action: action) -> Option<message> {
    Some(message::run(label.into(), action))
}

fn toggle(on: bool) -> (&'static str, &'static str) {
    if on { ("on", "off") } else { ("off", "on") }
}

pub fn bluetooth_glyph(status: &status) -> &'static str {
    if status.bluetooth.as_ref().is_some_and(|bluetooth| bluetooth.powered) { "\u{f00af}" } else { "\u{f00b2}" }
}

pub fn network_glyph(status: &status) -> &'static str {
    match status.link {
        link::ethernet => "\u{f0200}",
        link::none => "\u{f05aa}",
        link::wireless => "\u{f05a9}",
    }
}

pub fn volume_glyph(status: &status) -> &'static str {
    if status.audio.as_ref().is_some_and(|audio| audio.muted) { "\u{f0581}" } else { "\u{f057e}" }
}

pub fn title(deck: &deck, page: page) -> (&str, &str) {
    match page {
        page::app(index) => (&deck.config.app[index].icon, &deck.config.app[index].name),
        page::bluetooth => (bluetooth_glyph(&deck.status), "bluetooth"),
        page::network => (network_glyph(&deck.status), "network"),
        page::power => ("\u{f0425}", "power"),
        page::settings => ("\u{f0493}", "settings"),
    }
}

pub fn items(deck: &deck, page: page) -> Vec<item> {
    let status = &deck.status;
    match page {
        page::app(index) => {
            let state = if deck.running.contains(&index) { "running" } else { "" };
            vec![entry("\u{f040a}", "launch", state, Some(message::launch(index)))]
        }
        page::bluetooth => bluetooth(status, deck.device),
        page::network => network(status),
        page::power if status.power => [
            ("\u{f0425}", "power off", "PowerOff"),
            ("\u{f0709}", "reboot", "Reboot"),
            ("\u{f04b2}", "suspend", "Suspend"),
            ("\u{f0343}", "exit session", "TerminateSession"),
        ]
        .map(|(glyph, label, method)| entry(glyph, label, "", run(label, action::power(method))))
        .into(),
        page::power => Vec::new(),
        page::settings => settings(status),
    }
}

fn bluetooth(status: &status, device: Option<usize>) -> Vec<item> {
    let Some(bluetooth) = &status.bluetooth else { return Vec::new() };
    if let Some(device) = device.and_then(|index| bluetooth.devices.get(index)) {
        let (method, label) =
            if device.state == state::connected { ("Disconnect", "disconnect") } else { ("Connect", "connect") };
        let path = device.path.clone();
        return vec![
            entry("\u{f00b1}", label, &device.name, run(label, action::bluetooth(method, path.clone()))),
            entry("\u{f0a7a}", "forget", "", run("forget", action::bluetooth("forget", path))),
            entry("\u{f0141}", "back", "", Some(message::device(None))),
        ];
    }
    let (value, next) = toggle(bluetooth.powered);
    let mut items = vec![
        entry(
            bluetooth_glyph(status),
            "bluetooth",
            value,
            run("bluetooth", action::bluetooth(next, bluetooth.adapter.clone())),
        ),
        entry("\u{f0349}", "scan", "", run("scan", action::bluetooth("scan", bluetooth.adapter.clone()))),
    ];
    items.extend(bluetooth.devices.iter().enumerate().map(|(index, device)| {
        let state = match device.state {
            state::connected => "connected",
            state::new => "",
            state::paired => "paired",
        };
        let value = device.battery.map_or(state.into(), |battery| format!("{state}  {battery}%"));
        let select = match device.state {
            state::new => run("pair", action::bluetooth("Pair", device.path.clone())),
            _ => Some(message::device(Some(index))),
        };
        entry("\u{f00af}", &device.name, &value, select)
    }));
    items
}

fn network(status: &status) -> Vec<item> {
    let label = match status.link {
        link::ethernet => "ethernet",
        link::none => "not connected",
        link::wireless => "wireless",
    };
    let mut items = vec![entry(network_glyph(status), label, &status.interface, None)];
    let Some(wifi) = &status.wifi else { return items };
    let (value, next) = toggle(wifi.powered);
    items.push(entry("\u{f05a9}", "wi-fi", value, run("wi-fi", action::network(next, wifi.path.clone()))));
    items.push(entry("\u{f0349}", "scan", "", run("scan", action::network("Scan", wifi.path.clone()))));
    items.extend(wifi.networks.iter().map(|network| {
        let (value, select) = match network.active {
            true => ("connected".into(), run("disconnect", action::network("Disconnect", wifi.path.clone()))),
            false => {
                ("\u{f111}".repeat(network.signal), run("connect", action::network("Connect", network.path.clone())))
            }
        };
        entry("\u{f05a9}", &network.ssid, &value, select)
    }));
    items
}

fn settings(status: &status) -> Vec<item> {
    let mut items = Vec::new();
    if let Some(audio) = &status.audio {
        let value = if audio.muted { "muted".into() } else { format!("{}%", audio.volume) };
        items.push(item {
            left: run("volume", action::audio(audio.volume(false))),
            right: run("volume", action::audio(audio.volume(true))),
            ..entry(volume_glyph(status), "volume", &value, run("mute", action::audio(audio.mute())))
        });
        items.extend(audio.sinks.iter().map(|sink| {
            let value = if sink.default { "output" } else { "" };
            entry("\u{f04c3}", &sink.name, value, run("output", action::audio(audio.select(&sink.id))))
        }));
    }
    if let Some(profile) = &status.profile {
        let profiles = ["power-saver", "balanced", "performance"];
        let current = profiles.iter().position(|item| item == profile).unwrap_or(1);
        let step = |offset: usize| run("profile", action::profile(profiles[(current + offset) % 3].to_owned()));
        items.push(item { left: step(2), right: step(1), ..entry("\u{f04c5}", "power profile", profile, step(1)) });
    }
    if let Some(level) = status.brightness {
        let step = |level: u32| run("brightness", action::brightness(level.min(100)));
        items.push(item {
            left: step(level.saturating_sub(10)),
            right: step(level + 10),
            ..entry("\u{f00df}", "brightness", &format!("{level}%"), None)
        });
    }
    items
}
