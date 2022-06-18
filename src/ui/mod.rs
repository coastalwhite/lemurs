use log::info;

use std::io;
use std::process;

use crate::auth::{AuthUserInfo, AuthenticationError};
use crate::config::Config;
use crate::environment::{init_environment, set_xdg_env};
use crate::info_caching::{get_cached_username, set_cached_username};
use crate::post_login::{EnvironmentStartError, PostLoginEnvironment};
use status_message::StatusMessage;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame, Terminal,
};

mod input_field;
mod power_menu;
mod status_message;
mod switcher;

pub use input_field::{InputFieldDisplayType, InputFieldWidget};
pub use switcher::{SwitcherItem, SwitcherWidget};

use self::power_menu::PowerMenuWidget;
use self::status_message::{ErrorStatusMessage, InfoStatusMessage};

/// All the different modes for input
enum InputMode {
    /// Using the WM selector widget
    Switcher,

    /// Typing within the Username input field
    Username,

    /// Typing within the Password input field
    Password,

    /// Nothing selected
    Normal,
}

impl InputMode {
    /// Move to the next mode
    fn next(&mut self) {
        use InputMode::*;

        *self = match self {
            Normal => Switcher,
            Switcher => Username,
            Username => Password,
            Password => Password,
        }
    }

    /// Move to the previous mode
    fn prev(&mut self) {
        use InputMode::*;

        *self = match self {
            Normal => Normal,
            Switcher => Normal,
            Username => Switcher,
            Password => Username,
        }
    }
}

/// App holds the state of the application
pub struct LoginForm {
    /// Whether the application is running in preview mode
    preview: bool,

    /// The power menu
    power_menu_widget: PowerMenuWidget,

    /// The widget used for selection of the post-login environment
    switcher_widget: SwitcherWidget<PostLoginEnvironment>,

    /// Current value of the Username
    username_widget: InputFieldWidget,

    /// Current value of the Password
    password_widget: InputFieldWidget,

    /// Current input mode
    input_mode: InputMode,

    /// Error Message
    status_message: Option<StatusMessage>,

    /// The configuration for the app
    config: Config,
}

impl LoginForm {
    fn set_status_message(&mut self, status: impl Into<StatusMessage>) {
        let status = status.into();

        self.status_message = Some(status);
        // TODO: Redraw
    }

    fn clear_status_message(&mut self) {
        self.status_message = None;
        // TODO: Redraw
    }

    pub fn new(config: Config, preview: bool) -> LoginForm {
        let remember_username = config.username_field.remember_username;

        let preset_username = if remember_username {
            get_cached_username()
        } else {
            None
        }
        .unwrap_or(String::default());

        LoginForm {
            preview,
            power_menu_widget: PowerMenuWidget::new(config.power_controls.clone()),
            switcher_widget: SwitcherWidget::new(
                crate::post_login::get_envs()
                    .into_iter()
                    .map(|(title, content)| SwitcherItem::new(title, content))
                    .collect(),
                config.environment_switcher.clone(),
            ),
            username_widget: InputFieldWidget::new(
                InputFieldDisplayType::Echo,
                config.username_field.style.clone(),
                preset_username,
            ),
            password_widget: InputFieldWidget::new(
                InputFieldDisplayType::Replace(
                    config
                        .password_field
                        .content_replacement_character
                        .to_string(),
                ),
                config.password_field.style.clone().into(),
                String::default(),
            ),
            input_mode: InputMode::Normal,
            status_message: None,
            config,
        }
    }

    pub fn run<'a, B, A, S>(
        &mut self,
        terminal: &mut Terminal<B>,
        auth_fn: A,
        start_env_fn: S,
    ) -> io::Result<()>
    where
        B: Backend,
        A: Fn(String, String) -> Result<AuthUserInfo<'a>, AuthenticationError>,
        S: Fn(&AuthUserInfo, &PostLoginEnvironment) -> Result<(), EnvironmentStartError>,
    {
        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                match (key.code, &self.input_mode) {
                    (KeyCode::Enter, &InputMode::Password) => {
                        self.attempt_login(&auth_fn, &start_env_fn);
                    }
                    (KeyCode::Enter | KeyCode::Down, _) => {
                        self.input_mode.next();
                    }
                    (KeyCode::Up, _) => {
                        self.input_mode.prev();
                    }
                    (KeyCode::Tab, _) => {
                        if key.modifiers == KeyModifiers::SHIFT {
                            self.input_mode.prev();
                        } else {
                            self.input_mode.next();
                        }
                    }

                    // Esc is the overal key to get out of your input mode
                    (KeyCode::Esc, _) => {
                        if self.preview && matches!(self.input_mode, InputMode::Normal) {
                            info!("Pressed escape in preview mode to exit the application");
                            return Ok(());
                        }

                        self.input_mode = InputMode::Normal;
                    }

                    // For the different input modes the key should be passed to the corresponding
                    // widget.
                    (k, mode) => {
                        let status_message_opt = match mode {
                            &InputMode::Switcher => self.switcher_widget.key_press(k),
                            &InputMode::Username => self.username_widget.key_press(k),
                            &InputMode::Password => self.password_widget.key_press(k),
                            &InputMode::Normal => self.power_menu_widget.key_press(k),
                        };

                        // We don't wanna clear any existing error messages
                        if let Some(status_msg) = status_message_opt {
                            self.set_status_message(status_msg);
                        }
                    }
                };
            }
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        use Constraint::{Length, Min};

        let constraints = [
            Length(1),
            Length(1),
            Length(2),
            Length(1),
            Length(2),
            Length(3),
            Length(2),
            Length(3),
            Length(2),
            Length(1),
            Min(0),
        ];
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .horizontal_margin(2)
            .vertical_margin(1)
            .constraints(constraints.as_ref())
            .split(f.size());

        self.power_menu_widget.render(f, chunks[0]);

        self.switcher_widget
            .render(f, chunks[3], matches!(self.input_mode, InputMode::Switcher));

        self.username_widget
            .render(f, chunks[5], matches!(self.input_mode, InputMode::Username));

        self.password_widget
            .render(f, chunks[7], matches!(self.input_mode, InputMode::Password));

        // Display Status Message
        if let Some(status_message) = self.status_message {
            let error_widget = Paragraph::new(<&'static str>::from(status_message)).style(
                Style::default().fg(if status_message.is_error() {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            );

            f.render_widget(error_widget, chunks[9]);
        }
    }

    fn attempt_login<'a, A, S>(&mut self, auth_fn: A, start_env_fn: S)
    where
        A: Fn(String, String) -> Result<AuthUserInfo<'a>, AuthenticationError>,
        S: Fn(&AuthUserInfo, &PostLoginEnvironment) -> Result<(), EnvironmentStartError>,
    {
        let username = self.username_widget.get_content();
        let password = self.password_widget.get_content();

        // Fetch the selected post login environment
        let post_login_env = match self.switcher_widget.selected() {
            None => {
                self.set_status_message(ErrorStatusMessage::NoGraphicalEnvironment);
                return;
            }
            Some(selected) => selected,
        }.content.clone();

        self.set_status_message(InfoStatusMessage::Authenticating);
        let user_info = match auth_fn(username.clone(), password) {
            Err(err) => {
                self.set_status_message(ErrorStatusMessage::AuthenticationError(err));

                // Clear the password field
                self.password_widget.clear();

                return;
            }
            Ok(res) => res,
        };

        if self.config.username_field.remember_username {
            set_cached_username(&username);
        }

        self.set_status_message(InfoStatusMessage::LoggingIn);
        init_environment(&user_info.name, &user_info.dir, &user_info.shell);
        info!("Set environment variables.");

        set_xdg_env(user_info.uid, &user_info.dir, self.config.tty);
        info!("Set XDG environment variables");

        start_env_fn(&user_info, &post_login_env).unwrap_or_else(|_| {
            // NOTE: The error should already be reported
            self.set_status_message(ErrorStatusMessage::FailedGraphicalEnvironment);
        });

        self.clear_status_message();

        // Just to add explicitness that the user session is dropped here
        drop(user_info);
    }
}
