use super::{call_dbus_method, is_dbus_service_available, set_dbus_property};

const logind: &str = "org.freedesktop.login1";
const manager: &str = "org.freedesktop.login1.Manager";
const profiles: &str = "net.hadess.PowerProfiles";
const profiles_root: &str = "/net/hadess/PowerProfiles";
const root: &str = "/org/freedesktop/login1";

fn brightness_percent(current: u64, maximum: u64) -> u32 {
    (current.min(maximum) * 100 / maximum.max(1)) as u32
}

fn brightness_level(maximum: u64, level: u32) -> u32 {
    (maximum * u64::from(level.min(100)) / 100) as u32
}

fn backlight() -> Option<(String, u64, u64)> {
    let entry = crate::system::host::directory_entries("/sys/class/backlight").next()?;
    let read = |file| std::fs::read_to_string(entry.path().join(file)).ok()?.trim().parse().ok();
    Some((entry.file_name().to_string_lossy().into_owned(), read("brightness")?, read("max_brightness")?))
}

pub async fn perform_power_action(method: &str) -> Result<(), String> {
    if method == "TerminateSession" {
        let session = std::env::var("XDG_SESSION_ID").unwrap_or_default();
        return call_dbus_method(logind, root, manager, method, &(session,)).await;
    }
    call_dbus_method(logind, root, manager, method, &(true,)).await
}

pub async fn is_login_manager_available() -> bool {
    is_dbus_service_available(logind).await
}

pub async fn query_brightness() -> Option<u32> {
    let (_, current, maximum) = backlight()?;
    is_login_manager_available().await.then_some(brightness_percent(current, maximum))
}

pub async fn query_power_profile() -> Option<String> {
    if !is_dbus_service_available(profiles).await {
        return None;
    }
    let property = (profiles, "ActiveProfile");
    let value: zbus::zvariant::OwnedValue =
        call_dbus_method(profiles, profiles_root, "org.freedesktop.DBus.Properties", "Get", &property).await.ok()?;
    String::try_from(value).ok()
}

pub async fn set_brightness(level: u32) -> Result<(), String> {
    let (name, _, maximum) = backlight().ok_or("no backlight")?;
    let session: zbus::zvariant::OwnedObjectPath =
        call_dbus_method(logind, root, manager, "GetSessionByPID", &(std::process::id(),)).await?;
    let body = ("backlight", name, brightness_level(maximum, level));
    call_dbus_method(logind, session.as_str(), "org.freedesktop.login1.Session", "SetBrightness", &body).await
}

pub async fn set_profile(profile: String) -> Result<(), String> {
    set_dbus_property(profiles, profiles_root, profiles, "ActiveProfile", profile.into()).await
}

#[cfg(test)]
#[path = "test.rs"]
mod tests;
