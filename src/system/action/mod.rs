#[derive(Clone)]
pub enum action {
    audio(Vec<String>),
    bluetooth(&'static str, String),
    brightness(u32),
    network(&'static str, String),
    power(&'static str),
    profile(String),
    wifi_connect(String, String),
}
