use crossterm::event::KeyCode;
use log::error;
use serde::{de::Error, Deserialize};
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
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

macro_rules! var_replacement_strategy {
    ($vars:ident, $value:ident, $field_type:ty) => {
        <$field_type as VariableInsertable>::insert($value, $vars)?
    };
    ($vars:ident, $value:ident, $field_type:ty, $_:ty) => {
        $value.into_partial($vars)?
    };
}

macro_rules! toml_config_struct {
    ($struct_name:ident, $partial_struct_name:ident, $rough_name:ident, $($field_name:ident => $field_type:ty $([$par_field_type:ty, $rough_field_type:ty])?),+ $(,)?) => {
        #[derive(Debug, Clone, Deserialize)]
        struct $rough_name {
            $($field_name: Option<partial_struct_field!(PossibleVariable<$field_type>$(, $rough_field_type)?)>,)+
        }
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

        impl $rough_name {
            pub fn into_partial(self, variables: &Variables) -> Result<$partial_struct_name, VariableInsertionError> {
                Ok($partial_struct_name {
                    $(
                    $field_name: match self.$field_name {
                        Some(value) => Some(
                            var_replacement_strategy!(variables, value, $field_type$(, $par_field_type)?)
                        ),
                        None => None,
                    },
                    )+
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Variables(toml::value::Table);

toml_config_struct! { Config, PartialConfig, RoughConfig,
    tty => u8,

    main_log_path => String,
    client_log_path => String,

    do_log => bool,

    pam_service => String,
    system_shell => String,

    shell_login_flag => ShellLoginFlag,

    focus_behaviour => FocusBehaviour,

    background => BackgroundConfig [PartialBackgroundConfig, RoughBackgroundConfig],

    power_controls => PowerControlConfig [PartialPowerControlConfig, RoughPowerControlConfig],
    environment_switcher => SwitcherConfig [PartialSwitcherConfig, RoughSwitcherConfig],
    username_field => UsernameFieldConfig [PartialUsernameFieldConfig, RoughUsernameFieldConfig],
    password_field => PasswordFieldConfig [PartialPasswordFieldConfig, RoughPasswordFieldConfig],

    x11 => X11Config [PartialX11Config, RoughX11Config],
    wayland => WaylandConfig [PartialWaylandConfig, RoughWaylandConfig],
}

toml_config_struct! { BackgroundStyleConfig, PartialBackgroundStyleConfig, RoughBackgroundStyleConfig,
    color => String,
    show_border => bool,
    border_color => String,
}

toml_config_struct! { BackgroundConfig, PartialBackgroundConfig, RoughBackgroundConfig,
    show_background => bool,
    style => BackgroundStyleConfig [PartialBackgroundStyleConfig, RoughBackgroundStyleConfig],
}

toml_config_struct! { PowerControlConfig, PartialPowerControlConfig, RoughPowerControlConfig,
    hint_margin => u16,
    base_entries => PowerControlVec [PartialPowerControlVec, RoughPowerControlVec],
    entries => PowerControlVec [PartialPowerControlVec, RoughPowerControlVec],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct PowerControlVec(pub Vec<PowerControl>);
#[derive(Clone, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct PartialPowerControlVec(pub Vec<PartialPowerControl>);
#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
struct RoughPowerControlVec(pub Vec<RoughPowerControl>);

toml_config_struct! { PowerControl, PartialPowerControl, RoughPowerControl,
    hint => String,
    hint_color => String,
    hint_modifiers => String,
    key => String,
    cmd => String,
}

impl Default for PowerControl {
    fn default() -> Self {
        PowerControl {
            hint: "".to_string(),
            hint_color: "dark gray".to_string(),
            hint_modifiers: "".to_string(),
            key: "".to_string(),
            cmd: "true".to_string(),
        }
    }
}

toml_config_struct! { SwitcherConfig, PartialSwitcherConfig, RoughSwitcherConfig,
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

toml_config_struct! { InputFieldStyle, PartialInputFieldStyle, RoughInputFieldStyle,
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

toml_config_struct! { UsernameFieldConfig, PartialUsernameFieldConfig, RoughUsernameFieldConfig,
    remember => bool,
    style => InputFieldStyle [PartialInputFieldStyle, RoughInputFieldStyle],
}

toml_config_struct! { PasswordFieldConfig, PartialPasswordFieldConfig, RoughPasswordFieldConfig,
    content_replacement_character => char,
    style => InputFieldStyle [PartialInputFieldStyle, RoughInputFieldStyle],
}

toml_config_struct! { X11Config, PartialX11Config, RoughX11Config,
    x11_display => String,

    xserver_timeout_secs => u16,

    xserver_log_path => String,

    xserver_path => String,
    xauth_path => String,

    scripts_path => String,
}

toml_config_struct! { WaylandConfig, PartialWaylandConfig, RoughWaylandConfig,
    scripts_path => String,
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
        toml::from_str(include_str!("../extra/config.toml")).unwrap_or_else(|e| {
            eprintln!("Default configuration file cannot be properly parsed: {e}");
            process::exit(1);
        })
    }
}

impl PartialConfig {
    /// Facilitates the loading of the entire configuration
    pub fn from_file(
        path: &Path,
        variables: Option<&Variables>,
    ) -> Result<PartialConfig, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        match variables {
            Some(variables) => {
                let rough = toml::from_str::<RoughConfig>(&contents)?;
                Ok(rough.into_partial(variables)?)
            }
            None => Ok(toml::from_str::<PartialConfig>(&contents)?),
        }
    }
}

impl Variables {
    /// Facilitates the loading of the entire configuration
    pub fn from_file(path: &Path) -> Result<Variables, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        Ok(toml::from_str(&contents)?)
    }
}

trait VariableInsertable: Sized {
    const DEPTH_LIMIT: u32 = 10;

    fn insert(
        possible: PossibleVariable<Self>,
        variables: &Variables,
    ) -> Result<Self, VariableInsertionError> {
        Self::insert_with_depth(possible, variables, 0)
    }
    fn insert_with_depth(
        value: PossibleVariable<Self>,
        variables: &Variables,
        depth: u32,
    ) -> Result<Self, VariableInsertionError>;
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PossibleVariable<T> {
    Value(T),
    Variable(String),
}

impl<'de, T: Deserialize<'de>> TryFrom<toml::Value> for PossibleVariable<T> {
    type Error = &'static str;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        if let Ok(i) = value.clone().try_into() {
            return Ok(Self::Value(i));
        }

        match value {
            Value::String(s) => Ok(PossibleVariable::Variable(s)),
            v => Err(v.type_str()),
        }
    }
}

#[derive(Debug)]
enum VariableInsertionError {
    ImpossibleVariableCast {
        var_ident: String,
        expected_type: &'static str,
    },
    UnsetVariable {
        var_ident: String,
    },
    DepthLimitReached,
    InvalidType {
        expected: &'static str,
        gotten: &'static str,
    },
    UnexpectedVariableType {
        var_ident: String,
        expected_type: &'static str,
    },
}

impl Display for VariableInsertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableInsertionError::ImpossibleVariableCast {
                var_ident,
                expected_type,
            } => write!(
                f,
                "Impossible to use variable '{var_ident}' in string to cast to '{expected_type}'"
            ),
            VariableInsertionError::UnsetVariable { var_ident } => {
                write!(f, "Variable '{var_ident}' is not set")
            },
            VariableInsertionError::DepthLimitReached => {
                write!(f, "Variable evaluation reached the depth limit")
            },
            VariableInsertionError::InvalidType { expected, gotten } => write!(f, "Expected type '{expected}'. Got type '{gotten}'."),
            VariableInsertionError::UnexpectedVariableType { var_ident, expected_type } => write!(f, "Needed to use variable '{var_ident}' as a '{expected_type}', but was unable to cast it as such."),
        }
    }
}

impl PowerControlVec {
    pub fn merge_in_partial(&mut self, partial: PartialPowerControlVec) {
        *self = PowerControlVec(
            partial
                .0
                .into_iter()
                .map(|partial_elem| {
                    let mut elem = PowerControl::default();
                    elem.merge_in_partial(partial_elem);
                    elem
                })
                .collect::<Vec<PowerControl>>(),
        );
    }
}

impl RoughPowerControlVec {
    pub fn into_partial(
        self,
        variables: &Variables,
    ) -> Result<PartialPowerControlVec, VariableInsertionError> {
        self.0
            .into_iter()
            .map(|rough_elem| rough_elem.into_partial(variables))
            .collect::<Result<Vec<PartialPowerControl>, VariableInsertionError>>()
            .map(PartialPowerControlVec)
    }
}

impl std::error::Error for VariableInsertionError {}

macro_rules! non_string_var_insert {
    ($($type:ty [$type_str:literal]),+ $(,)?) => {
        $(
        impl VariableInsertable for $type {
            fn insert_with_depth(
                value: PossibleVariable<Self>,
                variables: &Variables,
                depth: u32,
            ) -> Result<Self, VariableInsertionError> {
                use VariableInsertionError as E;

                if depth == Self::DEPTH_LIMIT {
                    return Err(E::DepthLimitReached);
                }

                match value {
                    PossibleVariable::Variable(s) => {
                        // Ignore surrounding spaces
                        let s = s.trim();

                        let mut variter = VariableIterator::new(&s);

                        // No variable in string
                        let var = variter.next().ok_or(E::InvalidType {
                            expected: $type_str,
                            gotten: "string",
                        })?;

                        // Not whole string is variable
                        if var.span() != (0..s.len()) {
                            return Err(E::ImpossibleVariableCast {
                                var_ident: var.ident().to_string(),
                                expected_type: $type_str,
                            });
                        }

                        let value = <PossibleVariable<$type>>::try_from(
                            variables
                                .0
                                .get(var.ident())
                                .ok_or(E::UnsetVariable {
                                    var_ident: var.ident().to_string(),
                                })?
                                .clone(),
                        )
                        .map_err(|_| E::UnexpectedVariableType {
                            var_ident: var.ident().to_string(),
                            expected_type: $type_str,
                        })?;

                        Self::insert_with_depth(value, variables, depth + 1)
                    }
                    PossibleVariable::Value(b) => Ok(b),
                }
            }
        }
        )+
    };
}

non_string_var_insert! {
    bool ["boolean"],
    u8 ["unsigned 8-bit integer"],
    u16 ["unsigned 16-bit integer"],
    char ["character"],
    ShellLoginFlag ["shell login flag"],
    FocusBehaviour ["focus behavior"],
    SwitcherVisibility ["switcher visibility"],
}

impl VariableInsertable for String {
    fn insert_with_depth(
        value: PossibleVariable<Self>,
        variables: &Variables,
        depth: u32,
    ) -> Result<Self, VariableInsertionError> {
        use VariableInsertionError as E;

        if depth == Self::DEPTH_LIMIT {
            return Err(E::DepthLimitReached);
        }

        let mut s = match value {
            PossibleVariable::Value(s) | PossibleVariable::Variable(s) => s,
        };

        loop {
            let Some(var) = VariableIterator::new(&s).next() else {
                break;
            };

            let value = <PossibleVariable<String>>::try_from(
                variables
                    .0
                    .get(var.ident())
                    .ok_or(E::UnsetVariable {
                        var_ident: var.ident().to_string(),
                    })?
                    .clone(),
            )
            .map_err(|_| E::UnexpectedVariableType {
                var_ident: var.ident().to_string(),
                expected_type: "string",
            })?;

            let insertion = Self::insert_with_depth(value.clone(), variables, depth + 1)?;
            s.replace_range(var.span(), &insertion);
        }

        Ok(s)
    }
}

/// Iterator over variables in a given string
/// Assumes the presence of quotes
struct VariableIterator<'a> {
    inner: &'a str,
    offset: usize,
}

struct Variable<'a> {
    start: usize,
    ident: &'a str,
}

impl<'a> Variable<'a> {
    const START_SYMBOL: &'static str = "$";

    fn span(&self) -> std::ops::Range<usize> {
        self.start..self.start + Self::START_SYMBOL.len() + self.ident.len()
    }

    fn ident(&self) -> &str {
        self.ident
    }
}

impl<'a> VariableIterator<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            inner: text,
            offset: 0,
        }
    }
}
impl<'a> Iterator for VariableIterator<'a> {
    type Item = Variable<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let s = &self.inner[self.offset..];

        let start = match s.find(Variable::START_SYMBOL) {
            Some(position) => position,
            None => return None,
        };

        // skip the "$ pattern
        let s = &s[start + Variable::START_SYMBOL.len()..];

        // Find the first not variable token.
        let end = s
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(s.len());

        let start = self.offset + start;
        self.offset = start + Variable::START_SYMBOL.len() + end;

        let ident = &s[..end];

        Some(Variable { start, ident })
    }
}

#[cfg(test)]
mod tests {
    use super::VariableIterator;

    #[test]
    fn test_variable_iterator() {
        macro_rules! assert_var_iter {
            (
                $s:literal,
                ($($ident:literal),*)
            ) => {
                let variables: Vec<String> = VariableIterator::new($s).map(|v| v.ident().to_string()).collect();
                let idents: &[&str] = &[$($ident),*];

                eprintln!("variables = {variables:?}");
                eprintln!("ident = {idents:?}");

                assert_eq!(
                    &variables,
                    idents,
                );
            };
        }

        assert_var_iter!("", ());
        assert_var_iter!("abcdef", ());
        assert_var_iter!("$a", ("a"));
        assert_var_iter!("$a$b", ("a", "b"));
        assert_var_iter!("$a_c$b", ("a_c", "b"));
        assert_var_iter!("$a()$b", ("a", "b"));
        assert_var_iter!("$0    $1", ("0", "1"));
        assert_var_iter!("$var1    $var2    $var3  ", ("var1", "var2", "var3"));
    }
}
