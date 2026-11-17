use crate::config::AppConfig;
use std::process::{Child, Command};

pub fn launch(app: AppConfig) -> Result<Child, String> {
    let (program, arguments) = app.command.split_first().ok_or_else(|| format!("{} has no command", app.name))?;
    let mut command = Command::new(program);
    command.args(arguments);
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        command.envs([
            ("ELECTRON_OZONE_PLATFORM_HINT", "wayland"),
            ("GDK_BACKEND", "wayland,x11"),
            ("NIXOS_OZONE_WL", "1"),
            ("QT_QPA_PLATFORM", "wayland;xcb"),
            ("SDL_VIDEODRIVER", "wayland,x11"),
            ("XDG_SESSION_TYPE", "wayland"),
        ]);
    }
    command.envs(app.env).spawn().map_err(|error| format!("could not launch {}: {error}", app.name))
}

pub fn run(arguments: Vec<String>) -> Result<(), String> {
    let (program, arguments) = arguments.split_first().ok_or("action command is empty")?;
    let arguments = arguments.iter().map(|argument| expand_environment(argument)).collect::<Vec<_>>();
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    if status.success() { Ok(()) } else { Err(format!("{program} exited with {status}")) }
}

fn expand_environment(argument: &str) -> String {
    argument.strip_prefix('$').and_then(|name| std::env::var(name).ok()).unwrap_or_else(|| argument.to_owned())
}
