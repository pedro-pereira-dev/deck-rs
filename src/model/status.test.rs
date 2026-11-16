#[test]
fn calculates_cpu_load_from_idle_and_total_deltas() {
    assert_eq!(super::calculate_cpu_usage_percent((80, 100), (90, 150)), 80);
    assert_eq!(super::calculate_cpu_usage_percent((0, 0), (0, 0)), 100);
    assert_eq!(super::calculate_cpu_usage_percent((0, 0), (5, 4)), 0);
}

#[test]
fn formats_memory_and_uptime_without_exceeding_total_memory() {
    assert_eq!(super::format_memory_usage(2_097_152, 1_048_576), "1.0 / 2.0G");
    assert_eq!(super::format_memory_usage(2_097_152, 3_000_000), "0.0 / 2.0G");
    assert_eq!(super::format_uptime(90061), "25h 1m");
}

#[test]
fn parses_addresses_disk_usage_and_ping_output() {
    assert_eq!(super::parse_ip_addresses("lo UNKNOWN 127.0.0.1/8\neth0 UP 192.0.2.5/24\n"), "192.0.2.5");
    assert_eq!(super::parse_ip_addresses("lo UNKNOWN 127.0.0.1/8\n"), "");
    assert_eq!(
        super::format_disk_usage("Filesystem Used Size Use%\n/dev/root 4G 20G 20%\n"),
        "/dev/root / 4G / 20G / 20%"
    );
    assert_eq!(super::format_disk_usage(""), "");
    assert_eq!(super::parse_ping_latency("64 bytes time=1.25 ms"), "1.25");
    assert_eq!(super::parse_ping_latency("unavailable"), "-");
}
