use super::call;

const logind: &str = "org.freedesktop.login1";
const manager: &str = "org.freedesktop.login1.Manager";
const profiles: &str = "net.hadess.PowerProfiles";
const profiles_root: &str = "/net/hadess/PowerProfiles";
const root: &str = "/org/freedesktop/login1";

pub async fn action(method: &str) -> Result<(), String> {
    if method == "TerminateSession" {
        let session = std::env::var("XDG_SESSION_ID").unwrap_or_default();
        return call(logind, root, manager, method, &(session,)).await;
    }
    call(logind, root, manager, method, &(true,)).await
}

pub async fn available(backend: &str) -> bool {
    super::available(backend, "logind", logind).await
}

pub async fn brightness(backend: &str) -> Option<u32> {
    let (_, current, maximum) = backlight()?;
    available(backend).await.then_some((current * 100 / maximum.max(1)) as u32)
}

pub async fn profile(backend: &str) -> Option<String> {
    if !super::available(backend, "ppd", profiles).await {
        return None;
    }
    let property = (profiles, "ActiveProfile");
    let value: zbus::zvariant::OwnedValue =
        call(profiles, profiles_root, "org.freedesktop.DBus.Properties", "Get", &property).await.ok()?;
    String::try_from(value).ok()
}

pub async fn set_brightness(level: u32) -> Result<(), String> {
    let (name, _, maximum) = backlight().ok_or("no backlight")?;
    let session: zbus::zvariant::OwnedObjectPath =
        call(logind, root, manager, "GetSessionByPID", &(std::process::id(),)).await?;
    let body = ("backlight", name, (maximum * u64::from(level.min(100)) / 100) as u32);
    call(logind, session.as_str(), "org.freedesktop.login1.Session", "SetBrightness", &body).await
}

pub async fn set_profile(profile: String) -> Result<(), String> {
    super::set(profiles, profiles_root, profiles, "ActiveProfile", profile.into()).await
}

fn backlight() -> Option<(String, u64, u64)> {
    let entry = std::fs::read_dir("/sys/class/backlight").ok()?.flatten().next()?;
    let read = |file| std::fs::read_to_string(entry.path().join(file)).ok()?.trim().parse().ok();
    Some((entry.file_name().to_string_lossy().into_owned(), read("brightness")?, read("max_brightness")?))
}
