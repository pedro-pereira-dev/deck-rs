use super::run_command_and_capture_stdout;
use tokio::process::Command;

#[derive(Clone)]
pub struct sink {
    pub id: String,
    pub is_default: bool,
    pub name: String,
}

#[derive(Clone)]
pub struct audio {
    pub is_muted: bool,
    pub sinks: Vec<sink>,
    pub sources: Vec<sink>,
    pub tool: &'static str,
    pub volume: u32,
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn pactl_sinks(text: &str, info: &str, kind: &str) -> Vec<sink> {
    let prefix = format!("Default {kind}: ");
    let default = info.lines().find_map(|line| line.strip_prefix(prefix.as_str())).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(|name| sink { id: name.into(), is_default: name == default, name: name.into() })
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
            Some(sink {
                id: id.into(),
                is_default: line.starts_with('*'),
                name: name.split(" [").next()?.trim().into(),
            })
        })
        .collect()
}

impl audio {
    pub fn get_mute_command(&self) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", "set-sink-mute", "@DEFAULT_SINK@", "toggle"]),
            _ => args(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]),
        }
    }

    pub fn get_device_command(&self, id: &str, is_input: bool) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", if is_input { "set-default-source" } else { "set-default-sink" }, id]),
            _ => args(&["wpctl", "set-default", id]),
        }
    }

    pub fn get_volume_command(&self, level: u32) -> Vec<String> {
        match self.tool {
            "pactl" => args(&["pactl", "set-sink-volume", "@DEFAULT_SINK@", &format!("{level}%")]),
            _ => args(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{level}%")]),
        }
    }
}

// read volume, mute state, and available devices from pipewire or pulseaudio tools.
pub async fn query_audio_state() -> Option<audio> {
    let tool = ["wpctl", "pactl"].into_iter().find(|tool| super::is_executable_on_path(tool))?;
    Some(if tool == "pactl" {
        let (volume, info, muted, sinks, sources) = iced::futures::join!(
            run_command_and_capture_stdout(tool, &["get-sink-volume", "@DEFAULT_SINK@"]),
            run_command_and_capture_stdout(tool, &["info"]),
            run_command_and_capture_stdout(tool, &["get-sink-mute", "@DEFAULT_SINK@"]),
            run_command_and_capture_stdout(tool, &["list", "short", "sinks"]),
            run_command_and_capture_stdout(tool, &["list", "short", "sources"]),
        );
        audio {
            is_muted: muted.contains("yes"),
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
        let (volume, status) = iced::futures::join!(
            run_command_and_capture_stdout(tool, &["get-volume", "@DEFAULT_AUDIO_SINK@"]),
            run_command_and_capture_stdout(tool, &["status"])
        );
        audio {
            is_muted: volume.contains("MUTED"),
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

pub async fn run_audio_command(args: Vec<String>) -> Result<(), String> {
    let (program, args) = args.split_first().ok_or("empty command")?;
    let exit_status = Command::new(program).args(args).status().await.map_err(|error| error.to_string())?;
    exit_status.success().then_some(()).ok_or(format!("{program} failed"))
}

#[cfg(test)]
#[path = "test.rs"]
mod tests;
