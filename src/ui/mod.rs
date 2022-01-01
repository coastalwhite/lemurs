use log::{error, info};

use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, Receiver, channel};

use crate::graphical_environments::GraphicalEnvironment;
use crate::pam::{open_session, PamError};
use crate::{initrcs, X};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::backend::CrosstermBackend;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame, Terminal,
};

mod input_field;
mod window_manager_selector;

pub use input_field::{InputFieldDisplayType, InputFieldWidget};
pub use window_manager_selector::{WindowManager, WindowManagerSelectorWidget};

enum StatusMessageType {
    Error,
    Info,
}

enum StatusMessage {
    PamError(PamError),
    Authenticating,
    LoggingIn,
    FailedGraphicalEnvironment,
    FailedDesktop,
}

impl StatusMessage {
    /// Get the type of a [`StatusMessage`]
    fn message_type(status_message: &Self) -> StatusMessageType {
        match status_message {
            Self::PamError(_) | Self::FailedGraphicalEnvironment | Self::FailedDesktop => {
                StatusMessageType::Error
            }
            Self::Authenticating | Self::LoggingIn => StatusMessageType::Info,
        }
    }
}

/// All the different modes for input
enum InputMode {
    /// Using the WM selector widget
    WMSelect,
    
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
            Normal => WMSelect,
            WMSelect => Username,
            Username => Password,
            Password => Password,
        }
    }

    /// Move to the previous mode
    fn prev(&mut self) {
        use InputMode::*;

        *self = match self {
            Normal => Normal,
            WMSelect => Normal,
            Username => WMSelect,
            Password => Username,
        }
    }
}

/// App holds the state of the application
pub struct App {
    /// The widget used for selection of the window manager
    window_manager_widget: WindowManagerSelectorWidget,

    /// Current value of the Username
    username_widget: InputFieldWidget,

    /// Current value of the Password
    password_widget: InputFieldWidget,

    /// Current input mode
    input_mode: InputMode,

    /// Error Message
    status_message: Option<StatusMessage>,

    /// Authentication Receiver
    auth_channel: (Sender<(String, String, PathBuf)>, Receiver<Option<StatusMessage>>),
}

impl App {
    pub fn new() -> App {
        let (sender, auth_receiver) = channel();
        let (auth_sender, receiver) = channel();

        // Start the thread that will be handling the authentication
        std::thread::spawn(move || {
            loop {
                let (username, password, initrc_path) = auth_receiver.recv().unwrap();

                // TODO: Move this into the WindowManager struct to make it adjustable depending on
                // the window manager you are using
                let graphical_environment = X::new();
                login(
                    username,
                    password,
                    initrc_path,
                    &auth_sender,
                    graphical_environment,
                );
            }
        });
        
        App {
            window_manager_widget: WindowManagerSelectorWidget::new(initrcs::get_window_managers()),
            username_widget: InputFieldWidget::new("Login", InputFieldDisplayType::Echo),
            password_widget: InputFieldWidget::new("Password", InputFieldDisplayType::Replace('*')),
            input_mode: InputMode::Normal,
            status_message: None,
            auth_channel: (sender, receiver),
        }
    }
}

pub fn start() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    info!("UI booted up");

    Ok(terminal)
}

pub fn stop(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    info!("Reset terminal environment");

    Ok(())
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        let (snd, rcv) = &app.auth_channel;
        if let Ok(new_status_message) = rcv.try_recv() {
            app.status_message = new_status_message;
        }

        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, &app.input_mode) {
                (KeyCode::Enter, &InputMode::Password) => {
                    let username = app.username_widget.get_content();
                    let password = app.password_widget.get_content();
                    let initrc_path = app
                        .window_manager_widget
                        .selected()
                        .map(|selected| selected.initrc_path.clone())
                        .unwrap(); // TODO: Remove unwrap
                    
                    // TODO: If the Login was successful, the rendering of the UI should probably
                    // pause.
                    snd.send((username, password, initrc_path)).unwrap();
                }
                (KeyCode::Enter | KeyCode::Down, _) => {
                    app.input_mode.next();
                }
                (KeyCode::Up, _) => {
                    app.input_mode.prev();
                }
                (KeyCode::Tab, _) => {
                    if key.modifiers == KeyModifiers::SHIFT {
                        app.input_mode.prev();
                    } else {
                        app.input_mode.next();
                    }
                }

                // Esc is the overal key to get out of your input mode
                (KeyCode::Esc, _) => {
                    app.input_mode = InputMode::Normal;
                }

                // For the different input modes the key should be passed to the corresponding
                // widget.
                (k, &InputMode::WMSelect) => {
                    app.window_manager_widget.key_press(k);
                }
                (k, &InputMode::Username) => {
                    app.username_widget.key_press(k);
                }
                (k, &InputMode::Password) => {
                    app.password_widget.key_press(k);
                }
                _ => {}
            }
        }
    }
}

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    use Constraint::{Length, Min};

    let constraints = [
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

    app.window_manager_widget
        .render(f, chunks[2], matches!(app.input_mode, InputMode::WMSelect));

    app.username_widget
        .render(f, chunks[4], matches!(app.input_mode, InputMode::Username));

    app.password_widget
        .render(f, chunks[6], matches!(app.input_mode, InputMode::Password));

    // Display Status Message
    if let Some(status_message) = &app.status_message {
        use StatusMessage::*;

        let error_widget = Paragraph::new(match status_message {
            PamError(_) => "Authentication failed",
            LoggingIn => "Authentication successful. Logging in...",
            Authenticating => "Verifying credentials",
            FailedGraphicalEnvironment => "Failed booting into the graphical environment",
            FailedDesktop => "Failed booting into desktop environment",
        })
        .style(
            Style::default().fg(match StatusMessage::message_type(status_message) {
                StatusMessageType::Info => Color::Yellow,
                StatusMessageType::Error => Color::Red,
            }),
        );

        f.render_widget(error_widget, chunks[8]);
    }
}

fn login(
    username: String,
    password: String,
    initrc_path: PathBuf,
    status_send: &Sender<Option<StatusMessage>>,
    mut graphical_environment: X,
) {
    status_send.send(Some(StatusMessage::Authenticating)).expect("MSPC failed!");

    info!(
        "Login attempt for '{}' with '{}'",
        username,
        initrc_path.to_str().unwrap_or("Not Found")
    );

    let (authenticator, passwd_entry) = match open_session(username, password) {
        Err(pam_error) => {
            error!("Authentication failed"); // TODO: Improve this log
            status_send.send(Some(StatusMessage::PamError(pam_error))).expect("MSPC failed!");
            return;
        }
        Ok(res) => res,
    };

    status_send.send(Some(StatusMessage::LoggingIn)).expect("MSPC failed!");
    info!("Authentication successful. Booting up graphical environment");

    // TODO: This should probably be moved to the graphical_environment module somewhere.

    match graphical_environment.start(&passwd_entry) {
        Err(err) => {
            error!("Failed to boot graphical enviroment. Reason: '{}'", err);
            status_send.send(Some(StatusMessage::FailedGraphicalEnvironment)).expect("MSPC failed!");
            return;
        }
        _ => {}
    }

    info!("Graphical environment booted up successfully. Booting up desktop environment");

    match graphical_environment.desktop(initrc_path, &passwd_entry) {
        Err(err) => {
            error!("Failed to boot desktop environment. Reason: '{}'", err);
            status_send.send(Some(StatusMessage::FailedDesktop)).expect("MSPC failed!");
            return;
        }
        _ => {}
    }

    status_send.send(None).expect("MSPC failed!");
    info!("Desktop environment shutdown. Shutting down graphical enviroment");

    graphical_environment.stop();

    info!("Graphical environment shutdown. Logging out");

    // Logout
    drop(authenticator);
}
