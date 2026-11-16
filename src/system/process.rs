use std::path::PathBuf;

use crate::config::configuration_app;

pub async fn play_navigation_sound(sound: PathBuf) {
    let player = ["pw-play", "paplay"].into_iter().find(|player| super::is_executable_on_path(player));
    if let Some(Ok(mut child)) = player.map(|player| tokio::process::Command::new(player).arg(sound).spawn()) {
        let _ = child.wait().await;
    }
}

pub async fn launch_app(app: configuration_app) {
    let Some((program, args)) = app.command.split_first() else { return };
    let mut command = tokio::process::Command::new(program);
    command.args(args);
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
    if let Ok(mut child) = command.envs(&app.env).spawn() {
        let _ = child.wait().await;
    }
}
