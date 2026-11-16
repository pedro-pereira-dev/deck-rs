pub mod action;
pub mod audio;
pub mod bluetooth;
pub mod dbus;
pub mod host;
pub mod power;
pub mod process;
pub mod wifi;

use self::dbus::{
    call_dbus_method, find_interface_properties, is_dbus_service_available, managed_objects, property_value,
    set_dbus_property,
};

fn is_executable_in_directories(program: &str, paths: impl IntoIterator<Item = std::path::PathBuf>) -> bool {
    paths.into_iter().any(|directory| directory.join(program).is_file())
}

pub async fn run_command_and_capture_stdout(program: &str, args: &[&str]) -> String {
    let output = tokio::process::Command::new(program).args(args).output().await;
    output.map(|output| String::from_utf8_lossy(&output.stdout).into_owned()).unwrap_or_default()
}

pub fn is_executable_on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| is_executable_in_directories(program, std::env::split_paths(&path)))
}

pub async fn perform_action(action: crate::system::action::action) -> Result<(), String> {
    use crate::system::action::action;
    match action {
        action::audio(args) => audio::run_audio_command(args).await,
        action::bluetooth(method, path) => bluetooth::perform_bluetooth_action(method, path).await,
        action::brightness(level) => power::set_brightness(level).await,
        action::network(method, path) => wifi::perform_wifi_action(method, path).await,
        action::power(method) => power::perform_power_action(method).await,
        action::profile(profile) => power::set_profile(profile).await,
    }
}
