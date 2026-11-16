use std::fs::{DirEntry, read_to_string};
use std::sync::Arc;

use crate::config::configuration;
use crate::system::{self, audio, bluetooth, power, wifi};

// classify the active network link for icons and labels.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum link {
    ethernet,
    #[default]
    none,
    wireless,
}

// machine-wide statistics and the live values shown by the interface.
#[derive(Default)]
pub struct host_metrics {
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

// one snapshot of all queried system modules, clock, and network information.
#[derive(Default)]
pub struct system_snapshot {
    pub active_network_interface: String,
    pub audio: Option<audio::audio>,
    pub bluetooth: Option<bluetooth::bluetooth>,
    pub brightness: Option<u32>,
    pub date: String,
    pub host_metrics: host_metrics,
    pub is_power_available: bool,
    pub network_link: link,
    pub power_profile: Option<String>,
    pub time: String,
    pub wifi: Option<wifi::wifi>,
}

fn active_network_interface() -> (link, String) {
    let ignored = ["br", "docker", "lo", "tailscale", "tun", "veth", "virbr", "wg"];
    directory_entries("/sys/class/net")
        .map(|entry| (entry.path(), entry.file_name().to_string_lossy().into_owned()))
        .filter(|(_, name)| !ignored.iter().any(|prefix| name.starts_with(prefix)))
        .find(|(path, _)| read_to_string(path.join("operstate")).is_ok_and(|state| state.trim() == "up"))
        .map_or((link::none, String::new()), |(path, name)| {
            (if path.join("wireless").exists() { link::wireless } else { link::ethernet }, name)
        })
}

async fn run_command_and_capture_stdout(program: &str, args: &[&str]) -> String {
    system::run_command_and_capture_stdout(program, args).await.trim().to_owned()
}

fn read_trimmed_file(file_path: &str) -> String {
    read_to_string(file_path).unwrap_or_default().trim().to_owned()
}

fn cpu() -> (u64, u64) {
    let line = read_trimmed_file("/proc/stat");
    let values: Vec<u64> =
        line.lines().next().unwrap_or_default().split_whitespace().skip(1).flat_map(str::parse).collect();
    (values.get(3).copied().unwrap_or(0), values.iter().sum())
}

fn calculate_cpu_usage_percent(previous: (u64, u64), current: (u64, u64)) -> u32 {
    let total = current.1.saturating_sub(previous.1).max(1);
    100u64.saturating_sub(current.0.saturating_sub(previous.0) * 100 / total) as u32
}

fn format_memory_usage(total: u64, available: u64) -> String {
    let gib = |kib: u64| kib as f64 / 1048576.0;
    format!("{:.1} / {:.1}G", gib(total - available.min(total)), gib(total))
}

fn parse_ip_addresses(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with("lo"))
        .filter_map(|line| line.split_whitespace().nth(2)?.split('/').next())
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_disk_usage(text: &str) -> String {
    text.lines().nth(1).unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" / ")
}

fn parse_ping_latency(text: &str) -> String {
    text.split("time=").nth(1).and_then(|time| time.split_whitespace().next()).unwrap_or("-").into()
}

fn format_uptime(seconds: u64) -> String {
    format!("{}h {}m", seconds / 3600, seconds / 60 % 60)
}

async fn collect_host_metrics(host: &str) -> host_metrics {
    let (before, args) = (cpu(), ["-c1", "-W1", host]);
    let (address_output, disk_output, ping_output, _) = iced::futures::join!(
        run_command_and_capture_stdout("ip", &["-brief", "-4", "addr"]),
        run_command_and_capture_stdout("df", &["-h", "--output=used,size,pcent", "/"]),
        run_command_and_capture_stdout("ping", &args),
        tokio::time::sleep(std::time::Duration::from_millis(250)),
    );
    let after = cpu();
    let meminfo = read_trimmed_file("/proc/meminfo");
    let memory = ["MemTotal:", "MemAvailable:"].map(|key| {
        let line = meminfo.lines().find(|line| line.starts_with(key)).unwrap_or_default();
        line.split_whitespace().nth(1).and_then(|value| value.parse::<u64>().ok()).unwrap_or(0)
    });
    let mut displays: Vec<String> = directory_entries("/sys/class/drm")
        .filter(|entry| read_to_string(entry.path().join("status")).is_ok_and(|status| status.trim() == "connected"))
        .filter_map(|entry| Some(entry.file_name().to_str()?.split_once('-')?.1.to_owned()))
        .collect();
    displays.sort();
    let seconds =
        read_trimmed_file("/proc/uptime").split('.').next().and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
    let temperature = directory_entries("/sys/class/thermal")
        .filter_map(|entry| read_to_string(entry.path().join("temp")).ok()?.trim().parse::<u32>().ok())
        .max()
        .map(|value| value / 1000);
    host_metrics {
        address: parse_ip_addresses(&address_output),
        cpu: calculate_cpu_usage_percent(before, after),
        disk: format_disk_usage(&disk_output),
        displays: displays.join(" "),
        host: read_trimmed_file("/proc/sys/kernel/hostname"),
        kernel: read_trimmed_file("/proc/sys/kernel/osrelease"),
        memory: format_memory_usage(memory[0], memory[1]),
        ping: parse_ping_latency(&ping_output),
        temperature,
        uptime: format_uptime(seconds),
    }
}

pub async fn collect_system_snapshot(config: Arc<configuration>) -> system_snapshot {
    let (audio, bluetooth, brightness, clock, power_available, power_profile, wifi, host_metrics) = iced::futures::join!(
        audio::query_audio_state(),
        bluetooth::query_bluetooth_state(),
        power::query_brightness(),
        run_command_and_capture_stdout("date", &["+%H:%M|%A, %-d of %B %Y"]),
        power::is_login_manager_available(),
        power::query_power_profile(),
        wifi::query_wifi_state(),
        collect_host_metrics(&config.ui.ping),
    );
    let (time, date) = clock.split_once('|').unwrap_or_default();
    let (time, date) = (time.into(), date.into());
    let (network_link, active_network_interface) = active_network_interface();
    system_snapshot {
        active_network_interface,
        audio,
        bluetooth,
        brightness,
        date,
        host_metrics,
        is_power_available: power_available,
        network_link,
        power_profile,
        time,
        wifi,
    }
}

pub fn directory_entries(directory: &str) -> impl Iterator<Item = DirEntry> {
    std::fs::read_dir(directory).into_iter().flatten().flatten()
}

#[cfg(test)]
#[path = "host.test.rs"]
mod tests;
