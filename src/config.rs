use std::fs::File;
use std::io::{self, BufReader, Read};

use crossterm::event::KeyCode;
use log::error;
use serde::Deserialize;

use tui::style::{Color, Modifier};

pub fn get_color(color: &str) -> Color {
    if let Some(color) = str_to_color(color) {
        color
    } else {
        error!("Did not recognize the color '{}'", color);
        Color::White
    }
}

fn str_to_color(color: &str) -> Option<Color> {
    use Color::*;

    let c = color.to_lowercase();
    Some(match &c[..] {
        // TUI colors
        "black" => Black,
        "red" => Red,
        "green" => Green,
        "yellow" => Yellow,
        "blue" => Blue,
        "magenta" => Magenta,
        "cyan" => Cyan,
        "gray" => Gray,
        "dark gray" => DarkGray,
        "light red" => LightRed,
        "light green" => LightGreen,
        "light yellow" => LightYellow,
        "light blue" => LightBlue,
        "light magenta" => LightMagenta,
        "light cyan" => LightCyan,
        "white" => White,

        // Custom colors
        "orange" => Rgb(255, 127, 0),

        // Hex and unknown
        c => {
            if !c.starts_with("#") || c.len() != 7 {
                return None;
            }

            let r = hex::decode(&c[1..3])
                .ok()
                .and_then(|mut bytes| bytes.pop())?;
            let g = hex::decode(&c[3..5])
                .ok()
                .and_then(|mut bytes| bytes.pop())?;
            let b = hex::decode(&c[5..7])
                .ok()
                .and_then(|mut bytes| bytes.pop())?;

            Rgb(r, g, b)
        }
    })
}

fn get_modifier(modifier: &str) -> Option<Modifier> {
    let m = modifier.trim().to_lowercase();
    Some(match &m[..] {
        "bold" => Modifier::BOLD,
        "dim" => Modifier::DIM,
        "italic" => Modifier::ITALIC,
        "underlined" => Modifier::UNDERLINED,
        "slow blink" => Modifier::SLOW_BLINK,
        "rapid blink" => Modifier::RAPID_BLINK,
        "reversed" => Modifier::REVERSED,
        "crossed out" => Modifier::CROSSED_OUT,
        "hidden" => Modifier::HIDDEN,
        _ => return None,
    })
}

pub fn get_modifiers(modifiers: &str) -> Vec<Modifier> {
    let mut ms = Vec::new();

    for modifier in modifiers.split(",") {
        if let Some(modifier) = get_modifier(modifier) {
            ms.push(modifier);
        }
    }

    ms
}

pub fn get_key(key: &str) -> KeyCode {
    match key.trim() {
        "F1" => KeyCode::F(1),
        "F2" => KeyCode::F(2),
        "F3" => KeyCode::F(3),
        "F4" => KeyCode::F(4),
        "F5" => KeyCode::F(5),
        "F6" => KeyCode::F(6),
        "F7" => KeyCode::F(7),
        "F8" => KeyCode::F(8),
        "F9" => KeyCode::F(9),
        "F10" => KeyCode::F(10),
        "F11" => KeyCode::F(11),
        "F12" => KeyCode::F(12),
        // TODO: Add others
        _ => KeyCode::F(255),
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub preview: bool,
    pub power_options: PowerOptionsConfig,
    pub window_manager_selector: WMSelectorConfig,
    pub username_field: UsernameFieldConfig,
    pub password_field: PasswordFieldConfig,
}

#[derive(Clone, Deserialize)]
pub struct PowerOptionsConfig {
    pub allow_shutdown: bool,
    pub shutdown_hint: String,
    pub shutdown_hint_color: String,
    pub shutdown_hint_modifiers: String,
    pub shutdown_key: String,
    pub shutdown_cmd: String,

    pub allow_reboot: bool,
    pub reboot_hint: String,
    pub reboot_hint_color: String,
    pub reboot_hint_modifiers: String,
    pub reboot_key: String,
    pub reboot_cmd: String,

    pub hint_margin: u16,
}

#[derive(Clone, Deserialize)]
pub struct WMSelectorConfig {
    pub show_movers: bool,
    pub mover_color: String,
    pub mover_color_focused: String,

    pub mover_modifiers: String,
    pub mover_modifiers_focused: String,

    pub left_mover: String,
    pub right_mover: String,

    pub mover_margin: u16,

    pub selected_color: String,
    pub selected_color_focused: String,

    pub selected_modifiers: String,
    pub selected_modifiers_focused: String,

    pub show_neighbours: bool,
    pub neighbour_color: String,
    pub neighbour_color_focused: String,

    pub neighbour_modifiers: String,
    pub neighbour_modifiers_focused: String,

    pub neighbour_margin: u16,

    pub max_display_length: u16,

    pub no_envs_text: String,

    pub no_envs_color: String,
    pub no_envs_color_focused: String,

    pub no_envs_modifiers: String,
    pub no_envs_modifiers_focused: String,
}

#[derive(Clone, Deserialize)]
pub struct UsernameFieldConfig {
    pub show_title: bool,
    pub title: String,

    pub show_border: bool,

    pub title_color: String,
    pub title_color_focused: String,

    pub content_color: String,
    pub content_color_focused: String,

    pub border_color: String,
    pub border_color_focused: String,
}

#[derive(Clone, Deserialize)]
pub struct PasswordFieldConfig {
    pub show_title: bool,
    pub title: String,

    pub show_border: bool,

    pub title_color: String,
    pub title_color_focused: String,

    pub content_color: String,
    pub content_color_focused: String,
    pub content_replacement_character: char,

    pub border_color: String,
    pub border_color_focused: String,
}

impl From<PasswordFieldConfig> for UsernameFieldConfig {
    fn from(item: PasswordFieldConfig) -> Self {
        let PasswordFieldConfig {
            show_title,
            title,
            show_border,
            title_color,
            title_color_focused,
            content_color,
            content_color_focused,
            border_color,
            border_color_focused,
            ..
        } = item;

        UsernameFieldConfig {
            show_title,
            title,
            show_border,
            title_color,
            title_color_focused,
            content_color,
            content_color_focused,
            border_color,
            border_color_focused,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        toml::from_str(include_str!("../extra/config.toml")).expect("Default config incorrect!")
    }
}

impl Config {
    pub fn from_file(path: &str) -> io::Result<Config> {
        let file = File::open(path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        Ok(toml::from_str(&contents).expect("Given configuration file contains errors."))
    }
}
