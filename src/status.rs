use crate::config::config;
use crate::modules::{audio, bluetooth, network, power};
use std::fs::{DirEntry, read_to_string};
use std::sync::Arc;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum link {
    ethernet,
    #[default]
    none,
    wireless,
}

#[derive(Default)]
pub struct system {
    pub address: String,
    pub cpu: u32,
    pub disk: String,
    pub displays: String,
    pub host: String,
    pub kernel: String,
    pub memory: String,
    pub ping: String,
    pub temperature: Option<u32>,
    pub uptime: String,
}

#[derive(Default)]
pub struct status {
    pub audio: Option<audio::audio>,
    pub bluetooth: Option<bluetooth::bluetooth>,
    pub brightness: Option<u32>,
    pub date: String,
    pub interface: String,
    pub link: link,
    pub power: bool,
    pub profile: Option<String>,
    pub system: system,
    pub time: String,
    pub wifi: Option<network::wifi>,
}

pub async fn query(config: Arc<config>) -> status {
    let (audio, bluetooth, brightness, clock, power, profile, wifi, system) = iced::futures::join!(
        audio::query(&config.audio.backend),
        bluetooth::query(&config.bluetooth.backend),
        power::brightness(&config.brightness.backend),
        output("date", &["+%H:%M|%A, %-d of %B %Y"]),
        power::available(&config.power.backend),
        power::profile(&config.profile.backend),
        network::query(&config.network.backend),
        system(&config.ui.ping),
    );
    let (time, date) = clock.split_once('|').unwrap_or_default();
    let (time, date) = (time.into(), date.into());
    let (link, interface) = interfaces();
    status { audio, bluetooth, brightness, date, interface, link, power, profile, system, time, wifi }
}

async fn output(program: &str, args: &[&str]) -> String {
    crate::modules::output(program, args).await.trim().to_owned()
}

pub fn entries(path: &str) -> impl Iterator<Item = DirEntry> {
    std::fs::read_dir(path).into_iter().flatten().flatten()
}

fn read(path: &str) -> String {
    read_to_string(path).unwrap_or_default().trim().to_owned()
}

fn cpu() -> (u64, u64) {
    let line = read("/proc/stat");
    let values: Vec<u64> =
        line.lines().next().unwrap_or_default().split_whitespace().skip(1).flat_map(str::parse).collect();
    (values.get(3).copied().unwrap_or(0), values.iter().sum())
}

async fn system(host: &str) -> system {
    let (before, args) = (cpu(), ["-c1", "-W1", host]);
    let (address, disk, ping, _) = iced::futures::join!(
        output("ip", &["-brief", "-4", "addr"]),
        output("df", &["-h", "--output=used,size,pcent", "/"]),
        output("ping", &args),
        tokio::time::sleep(std::time::Duration::from_millis(250)),
    );
    let after = cpu();
    let total = after.1.saturating_sub(before.1).max(1);
    let meminfo = read("/proc/meminfo");
    let memory = ["MemTotal:", "MemAvailable:"].map(|key| {
        let line = meminfo.lines().find(|line| line.starts_with(key)).unwrap_or_default();
        line.split_whitespace().nth(1).and_then(|value| value.parse::<u64>().ok()).unwrap_or(0)
    });
    let gib = |kib: u64| kib as f64 / 1048576.0;
    let mut displays: Vec<String> = entries("/sys/class/drm")
        .filter(|entry| read_to_string(entry.path().join("status")).is_ok_and(|status| status.trim() == "connected"))
        .filter_map(|entry| Some(entry.file_name().to_str()?.split_once('-')?.1.to_owned()))
        .collect();
    displays.sort();
    let seconds = read("/proc/uptime").split('.').next().and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
    let temperature = entries("/sys/class/thermal")
        .filter_map(|entry| read_to_string(entry.path().join("temp")).ok()?.trim().parse::<u32>().ok())
        .max()
        .map(|value| value / 1000);
    system {
        address: address
            .lines()
            .filter(|line| !line.starts_with("lo"))
            .filter_map(|line| line.split_whitespace().nth(2)?.split('/').next())
            .collect::<Vec<_>>()
            .join(" "),
        cpu: (100 - after.0.saturating_sub(before.0) * 100 / total) as u32,
        disk: disk.lines().nth(1).unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" / "),
        displays: displays.join(" "),
        host: read("/proc/sys/kernel/hostname"),
        kernel: read("/proc/sys/kernel/osrelease"),
        memory: format!("{:.1} / {:.1}G", gib(memory[0] - memory[1].min(memory[0])), gib(memory[0])),
        ping: ping.split("time=").nth(1).and_then(|time| time.split_whitespace().next()).unwrap_or("-").into(),
        temperature,
        uptime: format!("{}h {}m", seconds / 3600, seconds / 60 % 60),
    }
}

fn interfaces() -> (link, String) {
    let ignored = ["br", "docker", "lo", "tailscale", "tun", "veth", "virbr", "wg"];
    entries("/sys/class/net")
        .map(|entry| (entry.path(), entry.file_name().to_string_lossy().into_owned()))
        .filter(|(_, name)| !ignored.iter().any(|prefix| name.starts_with(prefix)))
        .find(|(path, _)| read_to_string(path.join("operstate")).is_ok_and(|state| state.trim() == "up"))
        .map_or((link::none, String::new()), |(path, name)| {
            (if path.join("wireless").exists() { link::wireless } else { link::ethernet }, name)
        })
}
