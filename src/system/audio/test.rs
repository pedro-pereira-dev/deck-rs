#[test]
fn parses_pactl_sinks() {
    let sinks = super::pactl_sinks("42 alsa_output.pci-1\n43 usb_headset\n", "Default Sink: usb_headset\n", "Sink");
    assert_eq!(sinks.len(), 2);
    assert_eq!(sinks[0].name, "alsa_output.pci-1");
    assert!(sinks[1].is_default);
}

#[test]
fn parses_wpctl_sinks() {
    let sinks = super::wpctl_sinks("Audio\n ├─ Sinks:\n │  * 42. Speakers [vol: 0.50]\n", "Sinks:");
    assert_eq!(sinks.len(), 1);
    assert!(sinks[0].is_default);
    assert_eq!(sinks[0].id, "42");
    assert_eq!(sinks[0].name, "Speakers");
}

#[test]
fn builds_backend_specific_control_commands() {
    let pactl = super::audio { is_muted: false, sinks: Vec::new(), sources: Vec::new(), tool: "pactl", volume: 25 };
    let wpctl = super::audio { tool: "wpctl", ..pactl.clone() };

    assert_eq!(pactl.get_mute_command(), ["pactl", "set-sink-mute", "@DEFAULT_SINK@", "toggle"]);
    assert_eq!(pactl.get_device_command("source-1", true), ["pactl", "set-default-source", "source-1"]);
    assert_eq!(pactl.get_device_command("sink-1", false), ["pactl", "set-default-sink", "sink-1"]);
    assert_eq!(pactl.get_volume_command(40), ["pactl", "set-sink-volume", "@DEFAULT_SINK@", "40%"]);
    assert_eq!(wpctl.get_mute_command(), ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]);
    assert_eq!(wpctl.get_device_command("42", true), ["wpctl", "set-default", "42"]);
    assert_eq!(wpctl.get_volume_command(80), ["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "80%"]);
}

#[test]
fn parser_handles_empty_and_missing_defaults() {
    assert!(super::pactl_sinks("", "", "Sink").is_empty());
    let sinks = super::pactl_sinks("42 speaker\n", "", "Sink");
    assert_eq!(sinks.len(), 1);
    assert!(!sinks[0].is_default);
    assert!(super::wpctl_sinks("Audio\n ├─ Sinks:\n", "Sinks:").is_empty());
}
