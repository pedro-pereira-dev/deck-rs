use std::ffi::OsStr;
use std::process::Output;
use std::time::Duration;

#[derive(serde::Deserialize)]
pub struct app {
    pub command: Vec<String>,
    pub icon: String,
    pub name: String,
}

#[derive(Default, serde::Deserialize)]
struct config {
    #[serde(default)]
    app: Vec<app>,
}

pub fn env(name: &str) -> String {
    let default = match name {
        "DECK_CHANNEL" => "nixos-unstable",
        "DECK_PING" => "9.9.9.9",
        _ => "",
    };
    std::env::var(name).ok().filter(|value| !value.is_empty()).unwrap_or_else(|| default.into())
}

async fn exec(seconds: u64, args: &[impl AsRef<OsStr>]) -> Result<Output, String> {
    let (program, args) = args.split_first().ok_or("empty command")?;
    let output = tokio::process::Command::new(program).args(args).kill_on_drop(true).output();
    match tokio::time::timeout(Duration::from_secs(seconds), output).await {
        Ok(result) => result.map_err(|error| error.to_string()),
        Err(_) => Err("timed out".into()),
    }
}

pub async fn output<const n: usize>(seconds: u64, args: [&str; n]) -> String {
    exec(seconds, &args).await.map(|output| String::from_utf8_lossy(&output.stdout).into_owned()).unwrap_or_default()
}

pub async fn failure(seconds: u64, args: &[String]) -> Option<String> {
    let output = match exec(seconds, args).await {
        Ok(output) if output.status.success() => return None,
        Ok(output) => output,
        Err(error) => return Some(error),
    };
    let error = String::from_utf8_lossy(&output.stderr);
    let line = error.lines().rfind(|line| !line.trim().is_empty()).unwrap_or("error");
    Some(line.trim().chars().take(60).collect())
}

pub async fn launch(command: Vec<String>) {
    let Some((program, args)) = command.split_first() else {
        return;
    };
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
    if let Ok(mut child) = command.spawn() {
        let _ = child.wait().await;
    }
}

pub async fn logout() {
    let seat = output(3, ["loginctl", "show-seat", "seat0", "-p", "ActiveSession", "--value"]).await;
    let session =
        Some(seat.trim().to_string()).filter(|seat| !seat.is_empty()).or(std::env::var("XDG_SESSION_ID").ok());
    if let Some(session) = session {
        output(5, ["loginctl", "terminate-session", &session]).await;
    }
}

pub fn apps() -> Vec<app> {
    let directory = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| std::path::Path::new(&home).join(".config")))
        .unwrap_or_default();
    let config: config = std::fs::read_to_string(directory.join("deck-rs/apps.toml"))
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default();
    if !config.app.is_empty() {
        return config.app;
    }
    [
        ("steam", "\u{f04d3}", &["steam", "-gamepadui"][..]),
        ("jellyfin", "\u{f008}", &["jellyfin-desktop"]),
        ("grayjay", "\u{f008}", &["waydroid", "app", "launch", "com.futo.platformplayer"]),
    ]
    .into_iter()
    .map(|(name, icon, command)| app {
        command: command.iter().map(|arg| arg.to_string()).collect(),
        icon: icon.into(),
        name: name.into(),
    })
    .collect()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default().trim().into()
}

pub fn info() -> String {
    let generation = std::fs::read_link("/nix/var/nix/profiles/system")
        .ok()
        .and_then(|link| {
            let name = link.file_name()?.to_str()?.to_owned();
            Some(name.strip_prefix("system-")?.strip_suffix("-link")?.to_owned())
        })
        .unwrap_or_else(|| "-".into());
    format!(
        "deck {}   linux {}   nixos {} gen {generation}",
        env!("CARGO_PKG_VERSION"),
        read("/proc/sys/kernel/osrelease"),
        read("/run/current-system/nixos-version"),
    )
}

pub async fn updates() -> String {
    let (channel, repository) = (env("DECK_CHANNEL"), env("DECK_UPDATES"));
    let url = format!("https://channels.nixos.org/{channel}/git-revision");
    let (latest, configuration, remote) = tokio::join!(
        output(15, ["curl", "-fsSL", &url]),
        output(5, ["nixos-version", "--configuration-revision"]),
        async {
            match repository.is_empty() {
                true => String::new(),
                false => output(15, ["git", "ls-remote", &repository, "HEAD"]).await,
            }
        },
    );
    let version = read("/run/current-system/nixos-version");
    let revision = version.rsplit('.').next().unwrap_or_default();
    let (latest, configuration) = (latest.trim(), configuration.trim());
    let remote = remote.split_whitespace().next().unwrap_or_default();
    let nixpkgs = (!revision.is_empty() && !latest.is_empty()).then(|| !latest.starts_with(revision));
    let config = (!remote.is_empty() && !configuration.is_empty()).then(|| remote != configuration);
    let pending: Vec<_> = [(nixpkgs, "nixpkgs"), (config, "config")]
        .into_iter()
        .filter_map(|(outdated, name)| outdated?.then_some(name))
        .collect();
    match (pending.is_empty(), nixpkgs.or(config).is_some()) {
        (false, _) => format!("\u{f06b0} {} update", pending.join(" + ")),
        (true, true) => "\u{f012c} up to date".into(),
        (true, false) => String::new(),
    }
}

pub async fn ping() -> Option<f32> {
    let reply = output(3, ["ping", "-c1", "-W1", &env("DECK_PING")]).await;
    reply.split("time=").nth(1)?.split_whitespace().next()?.parse().ok()
}
