# deck-rs

Fullscreen ten-foot launcher for TV and handheld systems. The main view currently contains only a configurable clock
and date, leaving the rest of the Slint UI blank for incremental customization. Rendering uses the Slint winit backend
and FemtoVG.

## Structure

- `src/ui/theme.slint` contains shared theme properties, and `src/ui/window.slint` defines the blank window shell.
- `src/ui/components/` contains reusable clock, header, category icon, item row, item column, and back button components.
- `src/ui/views/` contains the blank main view and the retained example view.
- `src/config.rs` loads the bundled TOML defaults and optional user configuration.
- `src/menu.rs` builds categories and menu actions from the configuration.
- `src/navigation.rs` contains the small selection helper.
- `src/input_gamepad.rs` reads controller input through gilrs.
- `src/actions.rs` launches applications and configured commands.
- `src/main.rs` connects state, inputs, and the Slint view.

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
background = "#080b12"
background_image = ""
categories = ["games"]
favorites = ["steam"]
font_family = "Symbols Nerd Font Mono"
icons = { games = "\U000F0297" }
text = "#f3f5f8"

[ui.clock]
format = "%H:%M"
date_format = "%A, %-d %B %Y"
box_height_percent = 0.0
box_width_percent = 100.0
font_family = "sans-serif"
text_color = "#ffffff"
date_color = "#999999"
time_font_percent = 11.25
date_font_percent = 1.8
horizontal_alignment = "center"
vertical_alignment = "top"
horizontal_padding_percent = 3.375
line_spacing_percent = 3.0
top_offset_percent = 3.0

[ui.header]
category_font_percent = 6.5
date_font_percent = 1.8
date_top_percent = 17.25
left_padding_percent = 3.375
time_font_percent = 11.25
top_percent = 3.0

[ui.example]
back_button_font_percent = 3.5
back_button_height_percent = 8.0
back_button_left_percent = 40.0
back_button_top_percent = 46.0
back_button_width_percent = 20.0

[ui.xmb]
category_center_percent = 50.0
category_center_y_percent = 33.0
category_gap_percent = 7.875
category_icon_height_percent = 14.0
category_icon_percent = 5.0
category_icon_width_percent = 7.875
category_selected_icon_percent = 6.0
error_bottom_percent = 5.0
error_font_percent = 1.5
error_horizontal_padding_percent = 3.375
item_column_offset_percent = 5.625
item_column_width_percent = 25.3125
item_font_percent = 1.5
item_glyph_offset_percent = -2.8125
item_glyph_percent = 2.1
item_glyph_width_percent = 5.625
item_height_percent = 3.0
item_label_offset_percent = 2.8125
item_negative_far_step_percent = 5.0
item_negative_far_top_percent = 23.0
item_negative_near_step_percent = 20.0
item_positive_far_gap_percent = 2.5
item_positive_far_step_percent = 5.0
item_positive_near_step_percent = 7.5
item_selected_font_percent = 2.25
item_selected_glyph_percent = 3.15
item_selected_height_percent = 4.5
item_selected_y_percent = 43.0
selection_line_percent = 0.15
selection_line_offset_percent = 2.61
vertical_offset_percent = 0.0
viewport_top_percent = 33.0

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

`ui` properties control colors, typography, XMB sizing, spacing, categories, icons, and favorites. Clock settings live
under `[ui.clock]`. `format` and `date_format` use the `date` command's format syntax and default to 24-hour time and
weekday/day/month/year, respectively. Clock fonts, colors, family, alignment, padding, spacing, shared box size, and
offset are configurable. All `*_percent` values are percentage points, not multipliers. Horizontal positions, widths,
and padding use the full window width; vertical positions, heights, spacing, and font sizes use the full window height.
For example, `time_font_percent = 11.25` means 11.25% of the window height and `box_width_percent = 100` means the
full window width before padding constraints. A `box_height_percent` of `0` uses content-derived height. Alignment
accepts `left`, `center`, or `right` horizontally and `top`, `center`, or `bottom` vertically. Header, example view,
and XMB sizes follow the same axis-specific percentage convention, with controls grouped under `[ui.header]`,
`[ui.example]`, and `[ui.xmb]`. No global size scale or component-relative ratio is applied.
The reusable `Clock` component accepts separate date/time font sizes, font family, colors, horizontal alignment
(`left`, `center`, or `right`), vertical alignment (`top`, `center`, or `bottom`), horizontal padding, line spacing,
and top offset.
`input.keyboard` maps actions to Slint key names. `input.gamepad`
maps gilrs button names to navigation actions. `commands` maps built-in commands to argument arrays; use arrays rather
than shell strings. Commands are optional and a missing command is disabled.

The `MainView` currently shows only `Clock`; the header and XMB instances remain commented out in
`src/ui/views/main.slint`. Example and busy overlays remain commented out in `src/ui/window.slint`. Reusable components,
Rust-side menu actions, navigation state, keyboard/gamepad input, and configuration remain available as you build the
interface.

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
The flake supports x86_64-linux and aarch64-linux and exports the package, NixOS module, Home Manager module, and
settings package.
