use crossterm::event::KeyCode;
use log::error;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{de::Error, Deserialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::process;
use toml::Value;

use ratatui::style::{Color, Modifier};

#[derive(Debug)]
pub struct VarError {
    variable: String,
    pos: usize,
}

impl std::error::Error for VarError {}

impl Display for VarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Variable {} not found at position {}",
            self.variable, self.pos
        )
    }
}

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
            if !c.starts_with('#') || c.len() != 7 {
                return None;
            }

            let r = &c[1..3];
            let g = &c[3..5];
            let b = &c[5..7];

            let r = u8::from_str_radix(r, 16).ok()?;
            let g = u8::from_str_radix(g, 16).ok()?;
            let b = u8::from_str_radix(b, 16).ok()?;

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

    for modifier in modifiers.split(',') {
        if let Some(modifier) = get_modifier(modifier) {
            ms.push(modifier);
        }
    }

    ms
}

pub fn get_function_key(key: &str) -> Option<KeyCode> {
    Some(match key.trim() {
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
        _ => return None,
    })
}

pub fn get_key(key: &str) -> KeyCode {
    if let Some(fn_key) = get_function_key(key) {
        return fn_key;
    }

    KeyCode::F(255)
}

macro_rules! partial_struct_field {
    ($field_type:ty) => {
        $field_type
    };
    ($field_type:ty, $par_field_type:ty) => {
        $par_field_type
    };
}

macro_rules! merge_strategy {
    ($self:ident, $dest:ident, $src:expr) => {
        $self.$dest = $src
    };
    ($self:ident, $dest:ident, $src:expr, $_:ty) => {
        $self.$dest.merge_in_partial($src)
    };
}

macro_rules! toml_config_struct {
    ($struct_name:ident, $partial_struct_name:ident, $($field_name:ident => $field_type:ty $([$par_field_type:ty])?),+ $(,)?) => {
        #[derive(Debug, Clone, Deserialize)]
        pub struct $struct_name {
            $(pub $field_name: $field_type,)+
        }
        #[derive(Clone, Deserialize)]
        pub struct $partial_struct_name {
            $(pub $field_name: Option<partial_struct_field!($field_type$(, $par_field_type)?)>,)+
        }
        impl $struct_name {
            pub fn merge_in_partial(&mut self, partial: $partial_struct_name) {
                $(
                if let Some($field_name) = partial.$field_name {
                    merge_strategy!(self, $field_name, $field_name $(, $par_field_type)?);
                }
                )+
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VariablesConfig {
    #[serde(default)]
    pub variables: HashMap<String, toml::Value>,
}

toml_config_struct! { Config, PartialConfig,
    tty => u8,

    x11_display => String,
    xserver_timeout_secs => u16,

    main_log_path => String,
    client_log_path => String,
    xserver_log_path => String,

    do_log => bool,

    pam_service => String,

    shell_login_flag => ShellLoginFlag,

    focus_behaviour => FocusBehaviour,

    background => BackgroundConfig [PartialBackgroundConfig],

    power_controls => PowerControlConfig [PartialPowerControlConfig],
    environment_switcher => SwitcherConfig [PartialSwitcherConfig],
    username_field => UsernameFieldConfig [PartialUsernameFieldConfig],
    password_field => PasswordFieldConfig [PartialPasswordFieldConfig],
}

toml_config_struct! { BackgroundStyleConfig, PartialBackgroundStyleConfig,
    color => String,
    show_border => bool,
    border_color => String,
}

toml_config_struct! { BackgroundConfig, PartialBackgroundConfig,
    show_background => bool,
    style => BackgroundStyleConfig [PartialBackgroundStyleConfig],
}

toml_config_struct! { PowerControlConfig, PartialPowerControlConfig,
    allow_shutdown => bool,
    shutdown_hint => String,
    shutdown_hint_color => String,
    shutdown_hint_modifiers => String,
    shutdown_key => String,
    shutdown_cmd => String,

    allow_reboot => bool,
    reboot_hint => String,
    reboot_hint_color => String,
    reboot_hint_modifiers => String,
    reboot_key => String,
    reboot_cmd => String,

    hint_margin => u16,
}

toml_config_struct! { SwitcherConfig, PartialSwitcherConfig,
    switcher_visibility => SwitcherVisibility,
    toggle_hint => String,
    toggle_hint_color => String,
    toggle_hint_modifiers => String,

    include_tty_shell => bool,

    remember => bool,

    show_movers => bool,
    mover_color => String,
    mover_color_focused => String,

    mover_modifiers => String,
    mover_modifiers_focused => String,

    left_mover => String,
    right_mover => String,

    mover_margin => u16,

    selected_color => String,
    selected_color_focused => String,

    selected_modifiers => String,
    selected_modifiers_focused => String,

    show_neighbours => bool,
    neighbour_color => String,
    neighbour_color_focused => String,

    neighbour_modifiers => String,
    neighbour_modifiers_focused => String,

    neighbour_margin => u16,

    max_display_length => u16,

    no_envs_text => String,

    no_envs_color => String,
    no_envs_color_focused => String,

    no_envs_modifiers => String,
    no_envs_modifiers_focused => String,
}

toml_config_struct! { InputFieldStyle, PartialInputFieldStyle,
    show_title => bool,
    title => String,

    show_border => bool,

    title_color => String,
    title_color_focused => String,

    content_color => String,
    content_color_focused => String,

    border_color => String,
    border_color_focused => String,

    use_max_width => bool,
    max_width => u16,
}

toml_config_struct! { UsernameFieldConfig, PartialUsernameFieldConfig,
    remember => bool,
    style => InputFieldStyle [PartialInputFieldStyle],
}

toml_config_struct! { PasswordFieldConfig, PartialPasswordFieldConfig,
    content_replacement_character => char,
    style => InputFieldStyle [PartialInputFieldStyle],
}

#[derive(Debug, Clone, Deserialize)]
pub enum FocusBehaviour {
    #[serde(rename = "default")]
    FirstNonCached,
    #[serde(rename = "no-focus")]
    NoFocus,
    #[serde(rename = "environment")]
    Environment,
    #[serde(rename = "username")]
    Username,
    #[serde(rename = "password")]
    Password,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ShellLoginFlag {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "short")]
    Short,
    #[serde(rename = "long")]
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitcherVisibility {
    Visible,
    Hidden,
    Keybind(KeyCode),
}

/// Deserialise from a string of "visible", "hidden", or the keybind ("F1"-"F12")
impl<'de> Deserialize<'de> for SwitcherVisibility {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;

        Ok(match s {
            "visible" => Self::Visible,
            "hidden" => Self::Hidden,
            key => {
                let Some(keycode) = get_function_key(key) else {
                    return Err(D::Error::custom(
                        "Invalid key provided to toggle switcher visibility. Only F1-F12 are allowed"
                    ));

                };

                Self::Keybind(keycode)
            }
        })
    }
}

impl Default for Config {
    fn default() -> Config {
        toml::from_str(include_str!("../extra/config.toml")).unwrap_or_else(|_| {
            eprintln!("Default configuration file cannot be properly parsed");
            process::exit(1);
        })
    }
}

impl Config {
    /// Facilitates the loading of the entire configuration
    pub fn load(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
        let mut config = Config::default();

        let (config_str, var_config) = load_as_parts(path)?;
        let processed_config = apply_variables(config_str, &var_config)?;
        let partial = from_string::<PartialConfig, _>(&processed_config);
        config.merge_in_partial(partial);
        Ok(config)
    }
}

/// Substitutes variables present in the configuration string
fn apply_variables(config_str: String, var_config: &VariablesConfig) -> Result<String, VarError> {
    static VARIABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)")
            .expect("Failed to compile the variable substitution regex")
    });

    let mut output = String::new();
    let mut last = 0;
    let config_len = config_str.len();
    for (start, end, var) in VARIABLE_REGEX
        .find_iter(&config_str)
        .map(|m| (m.start(), m.end(), m.as_str()))
    {
        let var_ident = &var[1..var.len()];
        let var_val = var_config.variables.get(var_ident).ok_or(VarError {
            variable: var_ident.to_owned(),
            pos: start,
        })?;
        output.push_str(&config_str[last..start - 1]);

        // any case that is not a string, will not be wrapped in brackets
        if let Value::String(var_string) = var_val {
            output.push_str(&format!("\"{}\"", var_string));
        } else {
            output.push_str(&var_val.to_string());
        }

        last = end + 1;
    }

    if last != config_len {
        output.push_str(&config_str[last..config_len]);
    }

    Ok(output)
}

/// Helper function to facilitate immediate deserialization of variables along passing the original file content
fn load_as_parts(path: &Path) -> io::Result<(String, VariablesConfig)> {
    match read_to_string(path) {
        Ok(contents) => {
            let variables = from_string::<VariablesConfig, _>(&contents);
            Ok((contents, variables))
        }
        Err(err) => Err(err),
    }
}

fn read_to_string(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;

    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();

    buf_reader.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Generic configuration file loading
fn from_string<T: DeserializeOwned, S: AsRef<str>>(contents: S) -> T {
    match toml::from_str::<T>(contents.as_ref()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Given configuration file contains errors:");
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
