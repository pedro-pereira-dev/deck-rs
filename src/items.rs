use crate::deck::{message, nav, panel};
use crate::status::{link, state, status};

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

pub fn run(label: &str, args: &[&str]) -> Option<message> {
    Some(message::run(label.into(), args.iter().map(|arg| arg.to_string()).collect()))
}

fn command(args: &[&str]) -> Option<message> {
    run("working", args)
}

fn on(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

pub fn bluetooth_glyph(status: &status) -> &'static str {
    if status.bluetooth { "\u{f00af}" } else { "\u{f00b2}" }
}

pub fn network_glyph(status: &status) -> &'static str {
    match status.link {
        link::ethernet => "\u{f0200}",
        link::none => "\u{f05aa}",
        link::wireless => "\u{f05a9}",
    }
}

pub fn volume_glyph(status: &status) -> &'static str {
    if status.muted { "\u{f0581}" } else { "\u{f057e}" }
}

pub fn items(panel: panel, status: &status, device: Option<usize>) -> Vec<item> {
    match panel {
        panel::bluetooth => bluetooth(status, device),
        panel::network => network(status),
        panel::power => power(),
        panel::settings => settings(status),
    }
}

fn agent(label: &str, steps: &str) -> Option<message> {
    let script = format!(
        "y() {{ for _ in $(seq $1); do echo yes; sleep 1; done; }}; {{ echo 'agent KeyboardDisplay'; \
         echo default-agent; echo 'pairable on'; {steps}; echo quit; }} | bluetoothctl"
    );
    run(label, &["sh", "-c", &script])
}

fn bluetooth(status: &status, device: Option<usize>) -> Vec<item> {
    if let Some(device) = device.and_then(|index| status.devices.get(index)) {
        let (mac, name) = (&device.mac, &device.name);
        let (label, steps) = match device.state {
            state::connected => ("disconnect", format!("echo 'disconnect {mac}'; y 3")),
            _ => ("connect", format!("echo 'trust {mac}'; echo 'connect {mac}'; y 8")),
        };
        return vec![
            entry("\u{f00b1}", label, name, agent(&format!("{label}ing {name}"), &steps)),
            entry("\u{f0a7a}", "forget", "", run(&format!("forgetting {name}"), &["bluetoothctl", "remove", mac])),
            entry("\u{f0141}", "back", "", Some(message::nav(nav::back))),
        ];
    }
    let toggle = on(!status.bluetooth);
    let scan = run("scanning", &["bluetoothctl", "--timeout", "8", "scan", "on"]);
    let mut items = vec![
        entry(bluetooth_glyph(status), "bluetooth", on(status.bluetooth), command(&["bluetoothctl", "power", toggle])),
        entry("\u{f0349}", "scan", "", scan),
    ];
    items.extend(status.devices.iter().enumerate().map(|(index, device)| {
        let (mac, name, state) = (&device.mac, &device.name, device.state);
        let value = match device.battery {
            Some(level) => format!("{state}  {} {level}%", if level < 20 { "\u{f0083}" } else { "\u{f0079}" }),
            None => state.to_string(),
        };
        let glyph = if state == state::connected { "\u{f00b1}" } else { "\u{f00af}" };
        let select = match state {
            state::new => agent(
                &format!("pairing {name}"),
                &format!(
                    "echo 'scan on'; y 6; echo 'pair {mac}'; y 12; echo 'trust {mac}'; \
                     echo 'connect {mac}'; y 5; echo 'scan off'"
                ),
            ),
            _ => Some(message::device(index)),
        };
        entry(glyph, name, &value, select)
    }));
    items
}

fn network(status: &status) -> Vec<item> {
    let (glyph, station) = (network_glyph(status), status.station.as_str());
    let (label, value) = match status.link {
        link::ethernet => ("ethernet", format!("{}  connected", status.interface)),
        link::none => ("none", "not connected".into()),
        link::wireless => ("wireless", format!("{}  connected", status.interface)),
    };
    let mut items = vec![entry(glyph, label, &value, None)];
    if station.is_empty() {
        return items;
    }
    let power = ["iwctl", "device", station, "set-property", "Powered", on(!status.wifi)];
    items.push(entry("\u{f05a9}", "wi-fi", on(status.wifi), command(&power)));
    items.push(entry("\u{f0349}", "scan", "", run("scanning", &["iwctl", "station", station, "scan"])));
    items.extend(status.networks.iter().map(|network| match network.active {
        true => entry("\u{f05a9}", &network.ssid, "connected", command(&["iwctl", "station", station, "disconnect"])),
        false => entry(
            "\u{f05a9}",
            &network.ssid,
            &"\u{f111}".repeat(network.signal),
            command(&["iwctl", "station", station, "connect", &network.ssid]),
        ),
    }));
    items
}

fn power() -> Vec<item> {
    let action = |glyph: &str, label: &str, action: &str| {
        let script = format!(
            "systemctl --no-ask-password {action} || \
             /run/wrappers/bin/sudo -n /run/current-system/sw/bin/systemctl {action}"
        );
        entry(glyph, label, "", command(&["sh", "-c", &script]))
    };
    vec![
        action("\u{f0425}", "power off", "poweroff"),
        action("\u{f0709}", "reboot", "reboot"),
        action("\u{f04b2}", "suspend", "suspend"),
        entry("\u{f0343}", "exit session", "", Some(message::quit)),
    ]
}

fn settings(status: &status) -> Vec<item> {
    let sink = "@DEFAULT_AUDIO_SINK@";
    let volume = if status.muted { "muted".into() } else { format!("{}%", status.volume) };
    let profiles = ["power-saver", "balanced", "performance"];
    let current = profiles.iter().position(|profile| *profile == status.profile).unwrap_or(1);
    let profile = |step: usize| command(&["powerprofilesctl", "set", profiles[(current + step) % 3]]);
    let mut items = vec![item {
        left: command(&["wpctl", "set-volume", sink, "5%-"]),
        right: command(&["wpctl", "set-volume", "-l", "1", sink, "5%+"]),
        ..entry(volume_glyph(status), "volume", &volume, command(&["wpctl", "set-mute", sink, "toggle"]))
    }];
    if let Some(brightness) = status.brightness {
        items.push(item {
            left: command(&["brightnessctl", "set", "10%-"]),
            right: command(&["brightnessctl", "set", "+10%"]),
            ..entry("\u{f00df}", "display brightness", &format!("{brightness}%"), None)
        });
    }
    if !status.profile.is_empty() {
        items.push(item {
            left: profile(2),
            right: profile(1),
            ..entry("\u{f04c5}", "power profile", &status.profile, profile(1))
        });
    }
    items.extend(status.sinks.iter().map(|sink| {
        let value = if sink.default { "output" } else { "" };
        entry("\u{f04c3}", &sink.name, value, command(&["wpctl", "set-default", &sink.id]))
    }));
    items
}
