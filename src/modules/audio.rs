use super::output;
use tokio::process::Command;

#[derive(Clone)]
pub struct sink {
    pub default: bool,
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct audio {
    pub muted: bool,
    pub sinks: Vec<sink>,
    pub sources: Vec<sink>,
    pub tool: &'static str,
    pub volume: u32,
}

impl audio {
    pub fn mute(&self) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", "set-sink-mute", "@DEFAULT_SINK@", "toggle"]),
            _ => args(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]),
        }
    }

    pub fn select(&self, id: &str, input: bool) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", if input { "set-default-source" } else { "set-default-sink" }, id]),
            _ => args(&["wpctl", "set-default", id]),
        }
    }

    pub fn volume(&self, level: u32) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", "set-sink-volume", "@DEFAULT_SINK@", &format!("{level}%")]),
            _ => args(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{level}%")]),
        }
    }
}

pub async fn query(backend: &str) -> Option<audio> {
    let tool =
        ["wpctl", "pactl"].into_iter().find(|tool| (backend == "auto" || backend == *tool) && super::which(tool))?;
    Some(if tool == "pactl" {
        let (volume, info, muted, sinks, sources) = iced::futures::join!(
            output(tool, &["get-sink-volume", "@DEFAULT_SINK@"]),
            output(tool, &["info"]),
            output(tool, &["get-sink-mute", "@DEFAULT_SINK@"]),
            output(tool, &["list", "short", "sinks"]),
            output(tool, &["list", "short", "sources"]),
        );
        audio {
            muted: muted.contains("yes"),
            sinks: pactl_sinks(&sinks, &info, "Sink"),
            sources: pactl_sinks(&sources, &info, "Source")
                .into_iter()
                .filter(|source| !source.id.ends_with(".monitor"))
                .collect(),
            tool,
            volume: volume
                .split('/')
                .nth(1)
                .and_then(|value| value.trim().trim_end_matches('%').parse().ok())
                .unwrap_or(0),
        }
    } else {
        let (volume, status) =
            iced::futures::join!(output(tool, &["get-volume", "@DEFAULT_AUDIO_SINK@"]), output(tool, &["status"]));
        audio {
            muted: volume.contains("MUTED"),
            sinks: wpctl_sinks(&status, "Sinks:"),
            sources: wpctl_sinks(&status, "Sources:"),
            tool,
            volume: volume
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse::<f32>().ok())
                .map_or(0, |value| (value * 100.0).round() as u32),
        }
    })
}

pub async fn run(args: Vec<String>) -> Result<(), String> {
    let (program, args) = args.split_first().ok_or("empty command")?;
    let status = Command::new(program).args(args).status().await.map_err(|error| error.to_string())?;
    status.success().then_some(()).ok_or(format!("{program} failed"))
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn pactl_sinks(text: &str, info: &str, kind: &str) -> Vec<sink> {
    let prefix = format!("Default {kind}: ");
    let default = info.lines().find_map(|line| line.strip_prefix(prefix.as_str())).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(|name| sink { default: name == default, id: name.into(), name: name.into() })
        .collect()
}

fn wpctl_sinks(text: &str, section: &str) -> Vec<sink> {
    text.lines()
        .skip_while(|line| !line.contains(section))
        .skip(1)
        .map(|line| line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '*'))
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let (id, name) = line.trim_start_matches(['*', ' ']).split_once(". ")?;
            Some(sink { default: line.starts_with('*'), id: id.into(), name: name.split(" [").next()?.trim().into() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_pactl_sinks() {
        let sinks = super::pactl_sinks("42 alsa_output.pci-1\n43 usb_headset\n", "Default Sink: usb_headset\n", "Sink");
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[0].name, "alsa_output.pci-1");
        assert!(sinks[1].default);
    }

    #[test]
    fn parses_wpctl_sinks() {
        let sinks = super::wpctl_sinks("Audio\n ├─ Sinks:\n │  * 42. Speakers [vol: 0.50]\n", "Sinks:");
        assert_eq!(sinks.len(), 1);
        assert!(sinks[0].default);
        assert_eq!(sinks[0].id, "42");
        assert_eq!(sinks[0].name, "Speakers");
    }
}
