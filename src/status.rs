use crate::sys::output;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum link {
    ethernet,
    #[default]
    none,
    wireless,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum state {
    connected,
    paired,
    new,
}

pub struct device {
    pub battery: Option<u32>,
    pub mac: String,
    pub name: String,
    pub state: state,
}

pub struct network {
    pub active: bool,
    pub signal: usize,
    pub ssid: String,
}

pub struct sink {
    pub default: bool,
    pub id: String,
    pub name: String,
}

#[derive(Default)]
pub struct status {
    pub address: String,
    pub bluetooth: bool,
    pub brightness: Option<u32>,
    pub clock: String,
    pub devices: Vec<device>,
    pub interface: String,
    pub link: link,
    pub muted: bool,
    pub networks: Vec<network>,
    pub profile: String,
    pub sinks: Vec<sink>,
    pub station: String,
    pub volume: u32,
    pub wifi: bool,
}

impl std::fmt::Display for state {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            state::connected => "connected",
            state::new => "",
            state::paired => "paired",
        })
    }
}

pub async fn query() -> status {
    let (link, interface, station) = interfaces();
    let addr = ["ip", "-4", "-br", "addr", "show", "dev", &interface];
    let sink = "@DEFAULT_AUDIO_SINK@";
    let (address, clock, radio, networks, show, devices, volume, audio, brightness, profile) = tokio::join!(
        output(3, addr),
        output(3, ["date", "+%a %d %b   %H:%M"]),
        output(3, ["iwctl", "device", &station, "show"]),
        output(3, ["iwctl", "station", &station, "get-networks"]),
        output(3, ["bluetoothctl", "show"]),
        devices(),
        output(3, ["wpctl", "get-volume", sink]),
        output(3, ["wpctl", "status"]),
        output(3, ["brightnessctl", "-m"]),
        output(3, ["powerprofilesctl", "get"]),
    );
    let level: f32 = volume.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or_default();
    status {
        address: match link {
            link::none => String::new(),
            _ => address.split_whitespace().nth(2).and_then(|a| a.split('/').next()).unwrap_or_default().into(),
        },
        bluetooth: show.contains("Powered: yes"),
        brightness: brightness.trim().split(',').nth(3).and_then(|v| v.trim_end_matches('%').parse().ok()),
        clock: clock.trim().into(),
        devices,
        interface,
        link,
        muted: volume.contains("MUTED"),
        networks: wifi(&networks),
        profile: profile.trim().into(),
        sinks: sinks(&audio),
        station,
        volume: (level * 100.0).round() as u32,
        wifi: radio
            .lines()
            .map(plain)
            .any(|line| line.contains("Powered") && line.split_whitespace().last() == Some("on")),
    }
}

fn entries(path: &str) -> impl Iterator<Item = std::fs::DirEntry> {
    std::fs::read_dir(path).into_iter().flatten().flatten()
}

fn interfaces() -> (link, String, String) {
    let skip = ["br", "docker", "lo", "tailscale", "tun", "veth", "virbr", "wg"];
    let mut found: Vec<(bool, String, bool)> = entries("/sys/class/net")
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if skip.iter().any(|prefix| name.starts_with(prefix)) {
                return None;
            }
            let path = entry.path();
            let up = std::fs::read_to_string(path.join("operstate")).is_ok_and(|s| s.trim() == "up");
            Some((path.join("wireless").exists(), name, up))
        })
        .collect();
    found.sort();
    let station = found.iter().find(|i| i.0).map(|i| i.1.clone()).unwrap_or_default();
    match found.into_iter().find(|i| i.2) {
        Some((true, name, _)) => (link::wireless, name, station),
        Some((false, name, _)) => (link::ethernet, name, station),
        None => (link::none, String::new(), station),
    }
}

async fn devices() -> Vec<device> {
    let (all, connected, paired) = tokio::join!(
        output(3, ["bluetoothctl", "devices"]),
        output(3, ["bluetoothctl", "devices", "Connected"]),
        output(3, ["bluetoothctl", "devices", "Paired"]),
    );
    let has = |list: &str, mac: &str| list.lines().any(|line| line.split(' ').nth(1) == Some(mac));
    let mut devices: Vec<device> = all
        .lines()
        .filter_map(|line| line.strip_prefix("Device ")?.split_once(' '))
        .filter(|(mac, name)| name.replace('-', ":") != *mac)
        .map(|(mac, name)| device {
            battery: None,
            mac: mac.into(),
            name: name.into(),
            state: match (has(&connected, mac), has(&paired, mac)) {
                (true, _) => state::connected,
                (_, true) => state::paired,
                _ => state::new,
            },
        })
        .collect();
    devices.sort_by_key(|device| device.state == state::new);
    devices.truncate(10);
    let levels = iced::futures::future::join_all(devices.iter().map(|d| battery(d.state, &d.mac))).await;
    for (device, level) in devices.iter_mut().zip(levels) {
        device.battery = level;
    }
    devices
}

async fn battery(state: state, mac: &str) -> Option<u32> {
    if state != state::connected {
        return None;
    }
    let info = output(3, ["bluetoothctl", "info", mac]).await;
    let prefix = format!("hid-{}-battery", mac.to_lowercase());
    info.lines()
        .find_map(|line| line.trim().strip_prefix("Battery Percentage:"))
        .and_then(|value| value.split(['(', ')']).nth(1)?.parse().ok())
        .or_else(|| {
            let entry = entries("/sys/class/power_supply")
                .find(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))?;
            std::fs::read_to_string(entry.path().join("capacity")).ok()?.trim().parse().ok()
        })
}

fn plain(text: &str) -> String {
    let mut escape = false;
    text.chars()
        .filter(|&character| match (escape, character) {
            (_, '\x1b') => {
                escape = true;
                false
            }
            (true, 'm') => {
                escape = false;
                false
            }
            (escape, _) => !escape,
        })
        .collect()
}

fn wifi(text: &str) -> Vec<network> {
    text.lines()
        .skip_while(|line| !line.contains("Network name"))
        .skip(2)
        .filter_map(|line| {
            let lit = line.rsplit(' ').next()?.split("\x1b[1;90m").next()?;
            let line = plain(line);
            let mut fields: Vec<_> = line.split_whitespace().collect();
            let active = fields.first() == Some(&">");
            if active {
                fields.remove(0);
            }
            (fields.len() >= 3).then(|| network {
                active,
                signal: lit.matches('*').count(),
                ssid: fields[..fields.len() - 2].join(" "),
            })
        })
        .take(8)
        .collect()
}

fn sinks(text: &str) -> Vec<sink> {
    text.lines()
        .skip_while(|line| !line.contains("Sinks:"))
        .skip(1)
        .map(|line| line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '*'))
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let (id, name) = line.trim_start_matches(['*', ' ']).split_once(". ")?;
            Some(sink { default: line.starts_with('*'), id: id.into(), name: name.split(" [").next()?.trim().into() })
        })
        .collect()
}
