use crate::deck::{action, deck, message};
use crate::modules::bluetooth::state;
use crate::status::{link, status};

pub const check: &str = "\u{f012c}";

#[derive(Clone)]
pub struct item {
    pub glyph: String,
    pub label: String,
    pub select: Option<message>,
    pub sub: Vec<item>,
    pub value: String,
}

fn entry(glyph: &str, label: &str, value: &str, select: Option<message>) -> item {
    item { glyph: glyph.into(), label: label.into(), select, sub: Vec::new(), value: value.into() }
}

fn menu(glyph: &str, label: &str, value: &str, sub: Vec<item>) -> item {
    item { sub, ..entry(glyph, label, value, None) }
}

fn run(label: &str, action: action) -> Option<message> {
    Some(message::run(label.into(), action))
}

fn choice(glyph: &str, label: &str, current: bool, select: Option<message>) -> item {
    entry(glyph, label, if current { check } else { "" }, select)
}

fn toggle(glyph: &str, label: &str, on: bool, action: impl Fn(&'static str) -> action) -> item {
    let (value, next) = if on { ("on", "off") } else { ("off", "on") };
    entry(glyph, label, value, run(label, action(next)))
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

pub fn title<'a>(deck: &'a deck, page: &'a str) -> &'a str {
    match page {
        "favorites" => "\u{f04ce}",
        "home" => "\u{f02dc}",
        "power" => "\u{f0425}",
        "settings" => "\u{f0493}",
        page => deck.config.ui.icons.get(page).map_or("\u{f003b}", String::as_str),
    }
}

pub fn items(deck: &deck, page: &str) -> Vec<item> {
    let status = &deck.status;
    let app = |index: usize| {
        let app = &deck.config.app[index];
        let state = if deck.running.contains(&index) { "running" } else { "" };
        entry(&app.icon, &app.name, state, Some(message::launch(index)))
    };
    let suspend = || entry("\u{f04b2}", "suspend", "", run("suspend", action::power("Suspend")));
    match page {
        "favorites" => {
            let favorites = &deck.config.ui.favorites;
            let mut items: Vec<item> = favorites
                .iter()
                .filter_map(|name| deck.config.app.iter().position(|app| &app.name == name))
                .map(app)
                .collect();
            if items.is_empty() {
                items.push(entry("\u{ea6c}", "-", "", None));
            }
            items.push(suspend());
            items
        }
        "home" => home(status),
        "power" if !status.power => vec![suspend()],
        "power" => [
            ("\u{f0425}", "power off", "PowerOff"),
            ("\u{f0709}", "reboot", "Reboot"),
            ("\u{f0343}", "exit session", "TerminateSession"),
        ]
        .map(|(glyph, label, method)| entry(glyph, label, "", run(label, action::power(method))))
        .into_iter()
        .chain([suspend()])
        .collect(),
        "settings" => settings(status),
        page => (0..deck.config.app.len()).filter(|index| deck.config.app[*index].category == page).map(app).collect(),
    }
}

fn home(status: &status) -> Vec<item> {
    let system = &status.system;
    let temperature = system.temperature.map_or(String::new(), |value| format!("  {value}°C"));
    [
        ("\u{f048b}", "host", format!("{}  {}", system.host, system.kernel)),
        ("\u{f0954}", "uptime", system.uptime.clone()),
        ("\u{f0ee0}", "cpu", format!("{}%{temperature}", system.cpu)),
        ("\u{f035b}", "memory", system.memory.clone()),
        ("\u{f02ca}", "disk", system.disk.clone()),
        (network_glyph(status), "address", system.address.clone()),
        ("\u{f0379}", "displays", system.displays.clone()),
        ("\u{f0453}", "ping", format!("{} ms", system.ping)),
    ]
    .map(|(glyph, label, value)| entry(glyph, label, &value, None))
    .into()
}

fn settings(status: &status) -> Vec<item> {
    let mut items = Vec::new();
    if let Some(bluetooth) = &status.bluetooth {
        let adapter = &bluetooth.adapter;
        let mut sub = vec![
            toggle(bluetooth_glyph(status), "bluetooth", bluetooth.powered, |next| {
                action::bluetooth(next, adapter.clone())
            }),
            entry("\u{f0349}", "scan", "", run("scan", action::bluetooth("scan", adapter.clone()))),
        ];
        sub.extend(bluetooth.devices.iter().map(|device| {
            let path = &device.path;
            let state = match device.state {
                state::connected => "connected",
                state::new => "new",
                state::paired => "paired",
            };
            let value = device.battery.map_or(state.into(), |battery| format!("{state}  {battery}%"));
            let (method, label) = match device.state {
                state::connected => ("Disconnect", "disconnect"),
                state::new => ("Pair", "pair"),
                state::paired => ("Connect", "connect"),
            };
            let mut actions = vec![entry("\u{f00b1}", label, "", run(label, action::bluetooth(method, path.clone())))];
            if device.state != state::new {
                actions.push(entry(
                    "\u{f0a7a}",
                    "forget",
                    "",
                    run("forget", action::bluetooth("forget", path.clone())),
                ));
            }
            menu("\u{f00af}", &device.name, &value, actions)
        }));
        items.push(menu(bluetooth_glyph(status), "bluetooth", if bluetooth.powered { "on" } else { "off" }, sub));
    }
    let label = match status.link {
        link::ethernet => "ethernet",
        link::none => "not connected",
        link::wireless => "wireless",
    };
    let mut sub = Vec::new();
    if let Some(wifi) = &status.wifi {
        sub.push(toggle("\u{f05a9}", "wi-fi", wifi.powered, |next| action::network(next, wifi.path.clone())));
        sub.push(entry("\u{f0349}", "scan", "", run("scan", action::network("Scan", wifi.path.clone()))));
        sub.extend(wifi.networks.iter().map(|network| {
            let (value, label, action) = match network.active {
                true => ("connected".into(), "disconnect", action::network("Disconnect", wifi.path.clone())),
                false => ("●".repeat(network.signal), "connect", action::network("Connect", network.path.clone())),
            };
            menu("\u{f05a9}", &network.ssid, &value, vec![entry("\u{f05a9}", label, "", run(label, action))])
        }));
    }
    items.push(menu(network_glyph(status), "network", &format!("{label}  {}", status.interface), sub));
    if let Some(audio) = &status.audio {
        let value = if audio.muted { "muted".into() } else { format!("{}%", audio.volume) };
        let mut sub = vec![choice("\u{f0581}", "mute", audio.muted, run("mute", action::audio(audio.mute())))];
        sub.extend((0..=10).rev().map(|level| level * 10).map(|level| {
            let current = !audio.muted && audio.volume.abs_diff(level) < 5;
            choice("\u{f057e}", &format!("{level}%"), current, run("volume", action::audio(audio.volume(level))))
        }));
        items.push(menu(volume_glyph(status), "volume", &value, sub));
        for (glyph, label, input, devices) in
            [("\u{f04c3}", "output", false, &audio.sinks), ("\u{f036c}", "input", true, &audio.sources)]
        {
            let current = devices.iter().find(|device| device.default).map_or("", |device| &device.name);
            let sub = devices
                .iter()
                .map(|device| {
                    let select = run(label, action::audio(audio.select(&device.id, input)));
                    choice(glyph, &device.name, device.default, select)
                })
                .collect();
            items.push(menu(glyph, label, current, sub));
        }
    }
    if let Some(level) = status.brightness {
        let sub = (1..=10)
            .rev()
            .map(|step| step * 10)
            .map(|step| {
                let select = run("brightness", action::brightness(step));
                choice("\u{f00df}", &format!("{step}%"), level.abs_diff(step) < 5, select)
            })
            .collect();
        items.push(menu("\u{f00df}", "brightness", &format!("{level}%"), sub));
    }
    if let Some(profile) = &status.profile {
        let sub = ["power-saver", "balanced", "performance"]
            .map(|name| choice("\u{f04c5}", name, name == profile, run("profile", action::profile(name.into()))))
            .into();
        items.push(menu("\u{f04c5}", "power profile", profile, sub));
    }
    items
}
