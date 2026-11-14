use crate::config::config;
use crate::modules::{audio, bluetooth, network, power};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum link {
    ethernet,
    #[default]
    none,
    wireless,
}

#[derive(Clone, Default)]
pub struct status {
    pub audio: Option<audio::audio>,
    pub bluetooth: Option<bluetooth::bluetooth>,
    pub brightness: Option<u32>,
    pub clock: String,
    pub interface: String,
    pub link: link,
    pub power: bool,
    pub profile: Option<String>,
    pub wifi: Option<network::wifi>,
}

pub async fn query(config: config) -> status {
    let (audio, bluetooth, brightness, clock, power, profile, wifi) = iced::futures::join!(
        audio::query(&config.audio.backend),
        bluetooth::query(&config.bluetooth.backend),
        power::brightness(&config.brightness.backend),
        clock(),
        power::available(&config.power.backend),
        power::profile(&config.profile.backend),
        network::query(&config.network.backend),
    );
    let (link, interface) = interfaces();
    status { audio, bluetooth, brightness, clock, interface, link, power, profile, wifi }
}

async fn clock() -> String {
    let output = tokio::process::Command::new("date").arg("+%a %d %b  %H:%M").output().await;
    output.map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned()).unwrap_or_default()
}

fn interfaces() -> (link, String) {
    let ignored = ["br", "docker", "lo", "tailscale", "tun", "veth", "virbr", "wg"];
    std::fs::read_dir("/sys/class/net")
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| (entry.path(), entry.file_name().to_string_lossy().into_owned()))
        .filter(|(_, name)| !ignored.iter().any(|prefix| name.starts_with(prefix)))
        .find(|(path, _)| std::fs::read_to_string(path.join("operstate")).is_ok_and(|state| state.trim() == "up"))
        .map_or((link::none, String::new()), |(path, name)| {
            (if path.join("wireless").exists() { link::wireless } else { link::ethernet }, name)
        })
}
