use std::collections::BTreeMap;

fn configuration_ui_with_font(font: String) -> super::configuration_ui {
    super::configuration_ui {
        background: String::new(),
        categories: Vec::new(),
        favorites: Vec::new(),
        font,
        font_family: String::new(),
        icons: BTreeMap::new(),
        ping: String::new(),
        navigation_sound: String::new(),
    }
}

#[test]
fn overrides_merge_over_defaults() {
    let mut base: toml::Value = toml::from_str(super::default_configuration_file_path).unwrap();
    let overlay = toml::Value::Table(toml::toml! {
        app = [{ category = "games", name = "custom", command = ["custom"] }]
        ui.categories = ["games"]
    });

    super::merge_configurations(&mut base, overlay);
    let configuration: super::configuration = base.try_into().unwrap();

    assert_eq!(configuration.app.len(), 1);
    assert_eq!(configuration.app[0].category, "games");
    assert_eq!(configuration.ui.categories, ["games"]);
    assert_eq!(super::get_configured_categories(&configuration), ["Home", "Favorites", "games", "Settings", "Power"]);
}

#[test]
fn get_font_reads_the_configured_font_file() {
    let path = std::env::temp_dir().join(format!("deck-rs-font-{}.ttf", std::process::id()));
    let font = b"configured font";
    std::fs::write(&path, font).unwrap();
    let configuration_ui = configuration_ui_with_font(path.to_string_lossy().into_owned());

    let loaded_font = configuration_ui.get_font();

    std::fs::remove_file(path).unwrap();
    assert_eq!(loaded_font, font);
}

#[test]
fn get_font_falls_back_to_the_bundled_font_when_path_is_empty_or_missing() {
    let bundled_font = include_bytes!("../../assets/SymbolsNerdFontMono-Regular.ttf");
    let empty_path = configuration_ui_with_font(String::new());
    let missing_path = configuration_ui_with_font("missing-font-file.ttf".into());

    assert_eq!(empty_path.get_font(), bundled_font);
    assert_eq!(missing_path.get_font(), bundled_font);
}

#[test]
fn get_navigation_sound_reads_the_configured_sound_file() {
    let path = std::env::temp_dir().join(format!("deck-rs-sound-{}.wav", std::process::id()));
    let sound_bytes = b"configured sound";
    std::fs::write(&path, sound_bytes).unwrap();
    let mut configuration_ui = configuration_ui_with_font(String::new());
    configuration_ui.navigation_sound = path.to_string_lossy().into_owned();

    let loaded_sound = configuration_ui.get_navigation_sound();

    std::fs::remove_file(path).unwrap();
    assert_eq!(loaded_sound, sound_bytes);
}

#[test]
fn get_navigation_sound_falls_back_to_the_bundled_sound_when_path_is_empty_or_missing() {
    let bundled_sound = include_bytes!("../../assets/nav.wav");
    let empty_path = configuration_ui_with_font(String::new());
    let mut missing_path = configuration_ui_with_font(String::new());
    missing_path.navigation_sound = "missing-sound-file.wav".into();

    assert_eq!(empty_path.get_navigation_sound(), bundled_sound);
    assert_eq!(missing_path.get_navigation_sound(), bundled_sound);
}

#[test]
fn omitted_optional_fields_use_defaults() {
    let input = r#"
[ui]
background = ''
favorites = []
font = 'font.ttf'
font_family = 'font'
ping = 'host'
navigation_sound = 'sound.wav'

[[app]]
command = ['run']
name = 'app'
"#;

    let configuration: super::configuration = toml::from_str(input).unwrap();

    assert_eq!(configuration.app.len(), 1);
    assert_eq!(configuration.app[0].category, "");
    assert!(configuration.app[0].env.is_empty());
    assert_eq!(configuration.app[0].icon, "");
    assert!(configuration.ui.categories.is_empty());
    assert!(configuration.ui.icons.is_empty());
}

#[test]
fn configured_categories_ignore_builtin_names_and_case_insensitive_duplicates() {
    let mut configuration: super::configuration = toml::from_str(super::default_configuration_file_path).unwrap();

    configuration.ui.categories = ["tools", "home", "games", "power", "TOOLS", "favorites"].map(str::to_owned).into();

    assert_eq!(
        super::get_configured_categories(&configuration),
        ["Home", "Favorites", "tools", "games", "Settings", "Power"]
    );
}

#[test]
fn merge_recurses_into_tables_and_replaces_arrays_and_scalars() {
    let base_input = "[ui]\ncategories = ['games']\nfont = 'default'\nfont_family = 'sans'\n";
    let overlay_input = "[ui]\ncategories = ['tools']\nfont = 'custom'\n[ui.icons]\ntools = 'tools.svg'\n";

    let mut base: toml::Value = toml::from_str(base_input).unwrap();
    let overlay: toml::Value = toml::from_str(overlay_input).unwrap();

    super::merge_configurations(&mut base, overlay);

    assert_eq!(base["ui"]["categories"], toml::Value::Array(vec![toml::Value::String("tools".into())]));
    assert_eq!(base["ui"]["font"].as_str(), Some("custom"));
    assert_eq!(base["ui"]["font_family"].as_str(), Some("sans"));
    assert_eq!(base["ui"]["icons"]["tools"].as_str(), Some("tools.svg"));
}
