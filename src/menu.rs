use std::collections::HashSet;

use crate::config::configuration;
use crate::system::action::action;
use crate::system::bluetooth::state;
use crate::system::host::{link, system_snapshot};

pub const icon_check: &str = "\u{f012c}";

#[derive(Clone)]
pub struct item {
    pub activation: Option<item_activation>,
    pub children: Vec<item>,
    pub glyph: String,
    pub label: String,
    pub value: String,
}

#[derive(Clone)]
pub enum item_activation {
    launch(usize),
    run(String, action),
}

fn submenu_item(glyph: &str, label: &str, value: &str, activation: Option<item_activation>) -> item {
    item { activation, children: Vec::new(), glyph: glyph.into(), label: label.into(), value: value.into() }
}

fn menu_item(glyph: &str, label: &str, value: &str, children: Vec<item>) -> item {
    item { children, ..submenu_item(glyph, label, value, None) }
}

fn activate_item(label: &str, action: action) -> Option<item_activation> {
    Some(item_activation::run(label.into(), action))
}

fn choice_item(glyph: &str, label: &str, current: bool, activation: Option<item_activation>) -> item {
    submenu_item(glyph, label, if current { icon_check } else { "" }, activation)
}

fn percentage_choices(
    start: u32,
    glyph: &str,
    current: Option<u32>,
    label: &str,
    action: impl Fn(u32) -> action,
) -> impl Iterator<Item = item> {
    (start..=10).rev().map(|step| step * 10).map(move |step| {
        let selected = current.is_some_and(|current| current.abs_diff(step) < 5);
        choice_item(glyph, &format!("{step}%"), selected, activate_item(label, action(step)))
    })
}

fn toggle_item(glyph: &str, label: &str, on: bool, action: impl Fn(&'static str) -> action) -> item {
    let (value, next) = if on { ("on", "off") } else { ("off", "on") };
    submenu_item(glyph, label, value, activate_item(label, action(next)))
}

fn home_category_items(status: &system_snapshot) -> Vec<item> {
    let system = &status.host_metrics;
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
    .map(|(glyph, label, value)| submenu_item(glyph, label, &value, None))
    .into()
}

// build nested controls for bluetooth, networking, audio, brightness, and power profiles.
fn settings_category_items(status: &system_snapshot) -> Vec<item> {
    let mut items = Vec::new();
    if let Some(bluetooth) = &status.bluetooth {
        let adapter_object_path = &bluetooth.adapter_object_path;
        let mut sub = vec![
            toggle_item(bluetooth_glyph(status), "bluetooth", bluetooth.is_powered, |next| {
                action::bluetooth(next, adapter_object_path.clone())
            }),
            submenu_item(
                "\u{f0349}",
                "scan",
                "",
                activate_item("scan", action::bluetooth("scan", adapter_object_path.clone())),
            ),
        ];
        sub.extend(bluetooth.devices.iter().map(|device| {
            let object_path = &device.object_path;
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
            let mut actions = vec![submenu_item(
                "\u{f00b1}",
                label,
                "",
                activate_item(label, action::bluetooth(method, object_path.clone())),
            )];
            if device.state != state::new {
                actions.push(submenu_item(
                    "\u{f0a7a}",
                    "forget",
                    "",
                    activate_item("forget", action::bluetooth("forget", object_path.clone())),
                ));
            }
            menu_item("\u{f00af}", &device.name, &value, actions)
        }));
        items.push(menu_item(
            bluetooth_glyph(status),
            "bluetooth",
            if bluetooth.is_powered { "on" } else { "off" },
            sub,
        ));
    }
    let label = match status.network_link {
        link::ethernet => "ethernet",
        link::none => "not connected",
        link::wireless => "wireless",
    };
    let mut sub = Vec::new();
    if let Some(wifi) = &status.wifi {
        if !wifi.adapter_object_path.is_empty() {
            sub.push(toggle_item("\u{f05a9}", "wi-fi", wifi.is_powered, |next| {
                action::network(next, wifi.adapter_object_path.clone())
            }));
        }
        let scan_activation = wifi
            .station_object_path
            .as_ref()
            .and_then(|path| activate_item("scan", action::network("Scan", path.clone())));
        sub.push(submenu_item("\u{f0349}", "scan", "", scan_activation));
        sub.extend(wifi.networks.iter().map(|network| {
            let (value, label, action) = match network.is_active {
                true => (
                    "connected".into(),
                    "disconnect",
                    action::network("Disconnect", wifi.station_object_path.clone().unwrap_or_default()),
                ),
                false => (
                    "●".repeat(network.signal),
                    "connect",
                    action::wifi_connect(network.object_path.clone(), network.ssid.clone()),
                ),
            };
            menu_item(
                "\u{f05a9}",
                &network.ssid,
                &value,
                vec![submenu_item("\u{f05a9}", label, "", activate_item(label, action))],
            )
        }));
        if let Some(status) = &wifi.status {
            sub.push(submenu_item("\u{f05aa}", status, "", None));
        }
    }
    items.push(menu_item(
        network_glyph(status),
        "network",
        &format!("{label}  {}", status.active_network_interface),
        sub,
    ));
    if let Some(audio) = &status.audio {
        let value = if audio.is_muted { "muted".into() } else { format!("{}%", audio.volume) };
        let mut sub = vec![choice_item(
            "\u{f0581}",
            "mute",
            audio.is_muted,
            activate_item("mute", action::audio(audio.get_mute_command())),
        )];
        let level = (!audio.is_muted).then_some(audio.volume);
        sub.extend(percentage_choices(0, "\u{f057e}", level, "volume", |step| {
            action::audio(audio.get_volume_command(step))
        }));
        items.push(menu_item(volume_glyph(status), "volume", &value, sub));
        for (glyph, label, input, devices) in
            [("\u{f04c3}", "output", false, &audio.sinks), ("\u{f036c}", "input", true, &audio.sources)]
        {
            let current = devices.iter().find(|device| device.is_default).map_or("", |device| &device.name);
            let sub = devices
                .iter()
                .map(|device| {
                    let activation = activate_item(label, action::audio(audio.get_device_command(&device.id, input)));
                    choice_item(glyph, &device.name, device.is_default, activation)
                })
                .collect();
            items.push(menu_item(glyph, label, current, sub));
        }
    }
    if let Some(level) = status.brightness {
        let sub = percentage_choices(1, "\u{f00df}", Some(level), "brightness", action::brightness).collect();
        items.push(menu_item("\u{f00df}", "brightness", &format!("{level}%"), sub));
    }
    if let Some(profile) = &status.power_profile {
        let sub = ["power-saver", "balanced", "performance"]
            .map(|name| {
                choice_item("\u{f04c5}", name, name == profile, activate_item("profile", action::profile(name.into())))
            })
            .into();
        items.push(menu_item("\u{f04c5}", "power profile", profile, sub));
    }
    items
}

pub fn build_category_items(
    config: &configuration,
    status: &system_snapshot,
    running_apps: &HashSet<usize>,
    page: &str,
) -> Vec<item> {
    let app = |index: usize| {
        let app = &config.app[index];
        let state = if running_apps.contains(&index) { "running" } else { "" };
        submenu_item(&app.icon, &app.name, state, Some(item_activation::launch(index)))
    };
    let suspend = || submenu_item("\u{f04b2}", "suspend", "", activate_item("suspend", action::power("Suspend")));
    match page {
        "Favorites" => {
            let favorites = &config.ui.favorites;
            let mut items: Vec<item> = favorites
                .iter()
                .filter_map(|name| config.app.iter().position(|app| &app.name == name))
                .map(app)
                .collect();
            if items.is_empty() {
                items.push(submenu_item("\u{ea6c}", "-", "", None));
            }
            items.push(suspend());
            items
        }
        "Home" => home_category_items(status),
        "Power" if !status.is_power_available => vec![suspend()],
        "Power" => [
            ("\u{f0425}", "power off", "PowerOff"),
            ("\u{f0709}", "reboot", "Reboot"),
            ("\u{f0343}", "exit session", "TerminateSession"),
        ]
        .map(|(glyph, label, method)| submenu_item(glyph, label, "", activate_item(label, action::power(method))))
        .into_iter()
        .chain([suspend()])
        .collect(),
        "Settings" => settings_category_items(status),
        page => (0..config.app.len())
            .filter(|index| config.app[*index].category.eq_ignore_ascii_case(page))
            .map(app)
            .collect(),
    }
}

pub fn bluetooth_glyph(status: &system_snapshot) -> &'static str {
    if status.bluetooth.as_ref().is_some_and(|bluetooth| bluetooth.is_powered) { "\u{f00af}" } else { "\u{f00b2}" }
}

pub fn category_glyph<'a>(config: &'a configuration, page: &'a str) -> &'a str {
    match page {
        "Favorites" => "\u{f04ce}",
        "Home" => "\u{f02dc}",
        "Power" => "\u{f0425}",
        "Settings" => "\u{f0493}",
        page => config.ui.icons.get(page).map_or("\u{f003b}", String::as_str),
    }
}

pub fn network_glyph(status: &system_snapshot) -> &'static str {
    match status.network_link {
        link::ethernet => "\u{f0200}",
        link::none => "\u{f05aa}",
        link::wireless => "\u{f05a9}",
    }
}

pub fn volume_glyph(status: &system_snapshot) -> &'static str {
    if status.audio.as_ref().is_some_and(|audio| audio.is_muted) { "\u{f0581}" } else { "\u{f057e}" }
}
