# deck-rs

Fullscreen ten-foot launcher for TV and handheld systems. The horizontal cross menu uses Slint, with keyboard,
gamepad, and mouse controls. Rendering uses the Slint winit backend and FemtoVG.

## Structure

- `ui/` contains the Slint window, cross menu, theme, and menu item.
- `src/config.rs` loads the bundled TOML defaults and optional user configuration.
- `src/menu.rs` builds categories and menu actions from the configuration.
- `src/navigation.rs` contains the small selection helper.
- `src/input_gamepad.rs` reads controller input through gilrs.
- `src/actions.rs` launches applications and configured commands.
- `src/main.rs` connects state, inputs, and the Slint view.
- `material-1.1.0/` is the official Slint Material component library. This launcher uses its component library
  setup and can import its progress indicators and inputs when they are useful.

## Controls

| Action | Keyboard | Gamepad |
| --- | --- | --- |
| Move | Arrow keys | D-pad or left stick |
| Select | Enter or Space | South / Cross / A |
| Back | Escape | East / Circle / B |
| Menu | Home | Start or Guide |

Mouse click selects menu items and categories. The wheel changes the selected row, and right click goes back.
Launching an app hides the launcher until that process exits.

## Configuration

`assets/config.defaults.toml` is compiled into the binary. A user TOML overlays it at startup; tables merge and
arrays replace. Set `DECK_CONFIG`, or use `$XDG_CONFIG_HOME/deck-rs/config.toml` (default:
`~/.config/deck-rs/config.toml`). Invalid TOML is shown in the launcher.

```toml
[[app]]
name = "steam"
category = "games"
command = ["steam", "-gamepadui"]
icon = "\U000F04D3"
env = { GDK_BACKEND = "wayland,x11" }

[ui]
accent = "#8ab4f8"
background = "#080b12"
muted = "#9198a8"
panel = "#171d29"
selected = "#252f40"
text = "#f3f5f8"
subtitle = "Living room"
font_family = "Symbols Nerd Font Mono"
category_gap = 270
item_gap = 18
item_size = 30
categories = ["games"]
favorites = ["steam"]
icons = { games = "\U000F0297" }

[input]
stick_threshold = 0.6

[input.keyboard]
left = "LeftArrow"
right = "RightArrow"
up = "UpArrow"
down = "DownArrow"
select = "Enter"
back = "Escape"
menu = "Home"

[input.gamepad]
dpad_left = "left"
dpad_right = "right"
dpad_up = "up"
dpad_down = "down"
south = "select"
east = "back"
start = "menu"
mode = "menu"
select = "back"
stick_up = "up"
stick_down = "down"
stick_left = "left"
stick_right = "right"

[commands]
suspend = ["systemctl", "suspend"]
power_off = ["systemctl", "poweroff"]
reboot = ["systemctl", "reboot"]
bluetooth_on = ["bluetoothctl", "power", "on"]
bluetooth_off = ["bluetoothctl", "power", "off"]
wifi_on = ["nmcli", "radio", "wifi", "on"]
wifi_off = ["nmcli", "radio", "wifi", "off"]
audio_mute = ["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]
exit_session = ["loginctl", "terminate-session", "$XDG_SESSION_ID"]
```

`ui` properties control colors, typography, spacing, menu sizing, categories, icons, and favorites. `input.keyboard`
maps actions to Slint key names. `input.gamepad` maps gilrs button names to navigation actions. `commands` maps the
built-in settings and power entries to argument arrays; use arrays rather than shell strings. Commands are optional
and a missing command is disabled.

Categories are Home, Favorites, configured application categories, Settings, and Power. The UI currently provides
simple command entries for power, audio, Bluetooth, and Wi-Fi. Device discovery, Bluetooth pairing, Wi-Fi
credentials, host statistics, brightness, and power profiles are not part of this minimal port yet.

## Development

Use Rust 1.95 or newer, Cargo, and Nix for packaged builds and the development shell.

| Command | Description |
| --- | --- |
| `just build` | Build the launcher |
| `just run` | Run fullscreen |
| `just format` | Format Rust and Nix |
| `just lint` | Check formatting and Clippy |
| `just remote` | Sync and run on `REMOTE=user@host` |
| `just shot` | Capture a headless screenshot |
| `just watch` | Rebuild and restart after changes |

`PROFILE` defaults to `dev`; `RESOLUTION` defaults to `3840x2160`; `DELAY` defaults to `3` for screenshots.
Material Components 1.1.0 is included from the official release archive. The flake supports x86_64-linux and
aarch64-linux and exports the package, NixOS module, Home Manager module, and settings package.
