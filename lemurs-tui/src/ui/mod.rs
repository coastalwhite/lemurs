use log::{error, info, warn};

use std::io;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::config::{Config, FocusBehaviour};
use crate::info_caching::{get_cached_information, set_cache};
use lemurs::auth::{SessionUser, AuthenticationError};
use lemurs::session_environment::{EnvironmentStartError, SessionEnvironment};
use status_message::StatusMessage;

use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::CrosstermBackend;
use tui::{backend::Backend, Frame, Terminal};

mod chunks;
mod input_field;
mod power_menu;
mod status_message;
mod switcher;

use chunks::Chunks;
use input_field::{InputFieldDisplayType, InputFieldWidget};
use power_menu::PowerMenuWidget;
use status_message::{ErrorStatusMessage, InfoStatusMessage};
use switcher::{SwitcherItem, SwitcherWidget};

#[derive(Clone)]
struct LoginFormInputMode(Arc<Mutex<InputMode>>);

impl LoginFormInputMode {
    fn new(mode: InputMode) -> Self {
        Self(Arc::new(Mutex::new(mode)))
    }

    fn get_guard(&self) -> MutexGuard<InputMode> {
        let Self(mutex) = self;

        match mutex.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!("Lock failed. Reason: {}", err);
                std::process::exit(1);
            }
        }
    }

    fn get(&self) -> InputMode {
        *self.get_guard()
    }

    fn prev(&self) {
        self.get_guard().prev()
    }
    fn next(&self) {
        self.get_guard().next()
    }
    fn set(&self, mode: InputMode) {
        *self.get_guard() = mode;
    }
}

#[derive(Clone)]
struct LoginFormStatusMessage(Arc<Mutex<Option<StatusMessage>>>);

impl LoginFormStatusMessage {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    fn get_guard(&self) -> MutexGuard<Option<StatusMessage>> {
        let Self(mutex) = self;

        match mutex.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!("Lock failed. Reason: {}", err);
                std::process::exit(1);
            }
        }
    }

    fn get(&self) -> Option<StatusMessage> {
        *self.get_guard()
    }

    fn clear(&self) {
        *self.get_guard() = None;
    }
    fn set(&self, msg: impl Into<StatusMessage>) {
        *self.get_guard() = Some(msg.into());
    }
}

/// All the different modes for input
#[derive(Clone, Copy)]
enum InputMode {
    /// Using the env switcher widget
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

enum UIThreadRequest {
    Redraw,
    DisableTui,
    EnableTui,
    StopDrawing,
}

#[derive(Clone)]
struct Widgets {
    power_menu: PowerMenuWidget,
    environment: Arc<Mutex<SwitcherWidget<SessionEnvironment>>>,
    username: Arc<Mutex<InputFieldWidget>>,
    password: Arc<Mutex<InputFieldWidget>>,
}

impl Widgets {
    fn environment_guard(&self) -> MutexGuard<SwitcherWidget<SessionEnvironment>> {
        match self.environment.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!("Lock failed. Reason: {}", err);
                std::process::exit(1);
            }
        }
    }
    fn username_guard(&self) -> MutexGuard<InputFieldWidget> {
        match self.username.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!("Lock failed. Reason: {}", err);
                std::process::exit(1);
            }
        }
    }
    fn password_guard(&self) -> MutexGuard<InputFieldWidget> {
        match self.password.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!("Lock failed. Reason: {}", err);
                std::process::exit(1);
            }
        }
    }

    fn get_environment(&self) -> Option<(String, SessionEnvironment)> {
        self.environment_guard()
            .selected()
            .map(|s| (s.title.clone(), s.content.clone()))
    }
    fn environment_try_select(&self, title: &str) {
        self.environment_guard().try_select(title);
    }
    fn get_username(&self) -> String {
        self.username_guard().get_content()
    }
    fn set_username(&self, content: &str) {
        self.username_guard().set_content(content)
    }
    fn get_password(&self) -> String {
        self.password_guard().get_content()
    }
    fn clear_password(&self) {
        self.password_guard().clear()
    }
}

/// App holds the state of the application
#[derive(Clone)]
pub struct LoginForm {
    /// Whether the application is running in preview mode
    preview: bool,

    widgets: Widgets,

    /// The configuration for the app
    config: Config,
}

impl LoginForm {
    fn set_cache(&self) {
        let env_remember = self.config.environment_switcher.remember;
        let username_remember = self.config.username_field.remember;

        if !env_remember && !username_remember {
            info!("Nothing to cache.");
            return;
        }

        let selected_env = if self.config.environment_switcher.remember {
            self.widgets.get_environment().map(|(title, _)| title)
        } else {
            None
        };
        let username = self
            .config
            .username_field
            .remember
            .then_some(self.widgets.get_username());

        info!("Setting cached information");
        set_cache(selected_env.as_deref(), username.as_deref());
    }

    fn load_cache(&self) {
        let env_remember = self.config.environment_switcher.remember;
        let username_remember = self.config.username_field.remember;

        let cached = get_cached_information();

        if username_remember {
            if let Some(username) = cached.username() {
                info!("Loading username '{}' from cache", username);
                self.widgets.set_username(username);
            }
        }
        if env_remember {
            if let Some(env) = cached.environment() {
                info!("Loading environment '{}' from cache", env);
                self.widgets.environment_try_select(env);
            }
        }
    }

    pub fn new(config: Config, preview: bool) -> LoginForm {
        let mut session_environments =
            lemurs::session_environment::get_envs(config.environment_switcher.include_tty_shell);
        if session_environments.is_empty() {
            session_environments.push(SessionEnvironment::Shell);
        }

        LoginForm {
            preview,
            widgets: Widgets {
                power_menu: PowerMenuWidget::new(config.power_controls.clone()),
                environment: Arc::new(Mutex::new(SwitcherWidget::new(
                    session_environments
                        .into_iter()
                        .map(|env| {
                            let name = match &env {
                                SessionEnvironment::Shell => "TTY".to_string(),
                                SessionEnvironment::X11(session_script)
                                | SessionEnvironment::Wayland(session_script) => {
                                    session_script.name.clone()
                                }
                            };
                            SwitcherItem::new(name, env)
                        })
                        .collect(),
                    config.environment_switcher.clone(),
                ))),
                username: Arc::new(Mutex::new(InputFieldWidget::new(
                    InputFieldDisplayType::Echo,
                    config.username_field.style.clone(),
                    String::default(),
                ))),
                password: Arc::new(Mutex::new(InputFieldWidget::new(
                    InputFieldDisplayType::Replace(
                        config
                            .password_field
                            .content_replacement_character
                            .to_string(),
                    ),
                    config.password_field.style.clone(),
                    String::default(),
                ))),
            },
            config,
        }
    }

    pub fn run<'a, A, S>(
        self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        auth_fn: A,
        start_env_fn: S,
    ) -> io::Result<()>
    where
        A: Fn(String, String) -> Result<SessionUser<'a>, AuthenticationError>
            + std::marker::Send
            + 'static,
        S: Fn(&SessionEnvironment, &Config, &SessionUser) -> Result<(), EnvironmentStartError>
            + std::marker::Send
            + 'static,
    {
        self.load_cache();
        let input_mode = LoginFormInputMode::new(match self.config.focus_behaviour {
            FocusBehaviour::FirstNonCached => match (
                self.config.username_field.remember && !self.widgets.get_username().is_empty(),
                self.config.environment_switcher.remember
                    && self
                        .widgets
                        .get_environment()
                        .map(|(title, _)| !title.is_empty())
                        .unwrap_or(false),
            ) {
                (true, true) => InputMode::Password,
                (true, _) => InputMode::Username,
                _ => InputMode::Switcher,
            },
            FocusBehaviour::NoFocus => InputMode::Normal,
            FocusBehaviour::Environment => InputMode::Switcher,
            FocusBehaviour::Username => InputMode::Username,
            FocusBehaviour::Password => InputMode::Password,
        });
        let status_message = LoginFormStatusMessage::new();

        let power_menu = self.widgets.power_menu.clone();
        let environment = self.widgets.environment.clone();
        let username = self.widgets.username.clone();
        let password = self.widgets.password.clone();

        match terminal.draw(|f| {
            let layout = Chunks::new(f);
            login_form_render(
                f,
                layout,
                power_menu.clone(),
                environment.clone(),
                username.clone(),
                password.clone(),
                input_mode.get(),
                status_message.get(),
            );
        }) {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to draw. Reason: {}", err);
                std::process::exit(1);
            }
        }

        let event_input_mode = input_mode.clone();
        let event_status_message = status_message.clone();

        let (req_send_channel, req_recv_channel) = channel();
        std::thread::spawn(move || {
            let input_mode = event_input_mode;
            let status_message = event_status_message;

            let send_ui_request = |request: UIThreadRequest| match req_send_channel.send(request) {
                Ok(_) => {}
                Err(err) => warn!("Failed to send UI request. Reason: {}", err),
            };

            loop {
                if let Ok(Event::Key(key)) = event::read() {
                    match (key.code, input_mode.get()) {
                        (KeyCode::Enter, InputMode::Password) => {
                            if self.preview {
                                // This is only for demonstration purposes
                                status_message.set(InfoStatusMessage::Authenticating);
                                send_ui_request(UIThreadRequest::Redraw);
                                std::thread::sleep(Duration::from_secs(2));

                                status_message.set(InfoStatusMessage::LoggingIn);
                                send_ui_request(UIThreadRequest::Redraw);
                                std::thread::sleep(Duration::from_secs(2));

                                status_message.clear();
                                send_ui_request(UIThreadRequest::Redraw);
                            } else {
                                let environment =
                                    self.widgets.get_environment().map(|(_, content)| content);
                                let username = self.widgets.get_username();
                                let password = self.widgets.get_password();
                                let config = self.config.clone();

                                attempt_login(
                                    environment,
                                    username,
                                    password,
                                    config,
                                    status_message.clone(),
                                    send_ui_request,
                                    || self.widgets.clear_password(),
                                    || self.set_cache(),
                                    &auth_fn,
                                    &start_env_fn,
                                );
                            }
                        }
                        (KeyCode::Char('s'), InputMode::Normal) => self.set_cache(),
                        (KeyCode::Enter | KeyCode::Down, _) => {
                            input_mode.next();
                        }
                        (KeyCode::Up, _) => {
                            input_mode.prev();
                        }
                        (KeyCode::Tab, _) => {
                            if key.modifiers == KeyModifiers::SHIFT {
                                input_mode.prev();
                            } else {
                                input_mode.next();
                            }
                        }

                        // Esc is the overal key to get out of your input mode
                        (KeyCode::Esc, InputMode::Normal) => {
                            if self.preview {
                                info!("Pressed escape in preview mode to exit the application");
                                req_send_channel.send(UIThreadRequest::StopDrawing).unwrap();
                            }
                        }

                        (KeyCode::Esc, _) => {
                            input_mode.set(InputMode::Normal);
                        }

                        // For the different input modes the key should be passed to the corresponding
                        // widget.
                        (k, mode) => {
                            let status_message_opt = match mode {
                                InputMode::Switcher => {
                                    self.widgets.environment_guard().key_press(k)
                                }
                                InputMode::Username => self.widgets.username_guard().key_press(k),
                                InputMode::Password => self.widgets.password_guard().key_press(k),
                                InputMode::Normal => self.widgets.power_menu.key_press(k),
                            };

                            // We don't wanna clear any existing error messages
                            if let Some(status_msg) = status_message_opt {
                                status_message.set(status_msg);
                            }
                        }
                    };
                }

                send_ui_request(UIThreadRequest::Redraw);
            }
        });

        // Start the UI thread. This actually draws to the screen.
        //
        // This blocks until we actually call StopDrawing
        while let Ok(request) = req_recv_channel.recv() {
            match request {
                UIThreadRequest::Redraw => {
                    terminal
                        .draw(|f| {
                            let layout = Chunks::new(f);
                            login_form_render(
                                f,
                                layout,
                                power_menu.clone(),
                                environment.clone(),
                                username.clone(),
                                password.clone(),
                                input_mode.get(),
                                status_message.get(),
                            );
                        })
                        .unwrap();
                }
                UIThreadRequest::DisableTui => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        Clear(ClearType::All),
                        MoveTo(0, 0)
                    )?;
                    terminal.show_cursor()?;
                }
                UIThreadRequest::EnableTui => {
                    enable_raw_mode()?;
                    let mut stdout = io::stdout();
                    execute!(stdout, EnterAlternateScreen)?;
                    terminal.clear()?;
                }
                _ => break,
            }
        }

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn login_form_render<B: Backend>(
    frame: &mut Frame<B>,
    chunks: Chunks,
    power_menu: PowerMenuWidget,
    environment: Arc<Mutex<SwitcherWidget<SessionEnvironment>>>,
    username: Arc<Mutex<InputFieldWidget>>,
    password: Arc<Mutex<InputFieldWidget>>,
    input_mode: InputMode,
    status_message: Option<StatusMessage>,
) {
    power_menu.render(frame, chunks.power_menu);
    environment
        .lock()
        .unwrap_or_else(|err| {
            error!("Failed to lock post-login environment. Reason: {}", err);
            std::process::exit(1);
        })
        .render(
            frame,
            chunks.switcher,
            matches!(input_mode, InputMode::Switcher),
        );
    username
        .lock()
        .unwrap_or_else(|err| {
            error!("Failed to lock username. Reason: {}", err);
            std::process::exit(1);
        })
        .render(
            frame,
            chunks.username_field,
            matches!(input_mode, InputMode::Username),
        );
    password
        .lock()
        .unwrap_or_else(|err| {
            error!("Failed to lock password. Reason: {}", err);
            std::process::exit(1);
        })
        .render(
            frame,
            chunks.password_field,
            matches!(input_mode, InputMode::Password),
        );

    // Display Status Message
    StatusMessage::render(status_message, frame, chunks.status_message);
}

#[allow(clippy::too_many_arguments)]
fn attempt_login<'a, TR, PC, SC, A, S>(
    environment: Option<SessionEnvironment>,
    username: String,
    password: String,
    config: Config,
    status_message: LoginFormStatusMessage,
    send_ui_request: TR,
    password_clear: PC,
    set_cache: SC,
    auth_fn: A,
    start_env_fn: S,
) where
    TR: Fn(UIThreadRequest),
    PC: Fn(),
    SC: Fn(),
    A: Fn(String, String) -> Result<SessionUser<'a>, AuthenticationError>,
    S: Fn(&SessionEnvironment, &Config, &SessionUser) -> Result<(), EnvironmentStartError>,
{
    // Fetch the selected post login environment
    let post_login_env = match environment {
        None => {
            status_message.set(ErrorStatusMessage::NoGraphicalEnvironment);
            send_ui_request(UIThreadRequest::Redraw);
            return;
        }
        Some(selected) => selected,
    };

    status_message.set(InfoStatusMessage::Authenticating);
    send_ui_request(UIThreadRequest::Redraw);

    let user_info = match auth_fn(username, password) {
        Err(err) => {
            status_message.set(ErrorStatusMessage::AuthenticationError(err));

            // Clear the password field
            password_clear();
            send_ui_request(UIThreadRequest::Redraw);

            return;
        }
        Ok(res) => res,
    };

    // Remember username for next time
    set_cache();

    status_message.set(InfoStatusMessage::LoggingIn);
    send_ui_request(UIThreadRequest::Redraw);

    // Disable the rendering of the login manager
    send_ui_request(UIThreadRequest::DisableTui);

    // NOTE: if this call is succesful, it blocks the thread until the environment is
    // terminated
    start_env_fn(&post_login_env, &config, &user_info).unwrap_or_else(|_| {
        error!("Starting post-login environment failed");
        status_message.set(ErrorStatusMessage::FailedGraphicalEnvironment);
        send_ui_request(UIThreadRequest::Redraw);
    });

    // Enable the rendering of the login manager
    send_ui_request(UIThreadRequest::EnableTui);

    status_message.clear();
    send_ui_request(UIThreadRequest::Redraw);

    // Just to add explicitness that the user session is dropped here
    drop(user_info);
}
