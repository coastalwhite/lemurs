use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame, Terminal,
};

mod graphical_environments;
mod ui;
mod pam;
mod initrcs;
use graphical_environments::{GraphicalEnvironment, X};
use ui::{InputFieldDisplayType, InputFieldWidget, WindowManagerSelectorWidget};

enum InputMode {
    WindowManager,
    Username,
    Password,
    Normal,
}

/// App holds the state of the application
struct App {
    /// The widget used for selection of the window manager
    window_manager_widget: WindowManagerSelectorWidget,

    /// Current value of the Username
    username_widget: InputFieldWidget,

    /// Current value of the Password
    password_widget: InputFieldWidget,

    /// Current input mode
    input_mode: InputMode,

    /// Error Message
    error_msg: Option<AuthError>,

    graphical_environment: X,
}

impl Default for App {
    fn default() -> App {
        App {
            window_manager_widget: WindowManagerSelectorWidget::new(initrcs::get_window_managers()),
            username_widget: InputFieldWidget::new("Username", InputFieldDisplayType::Echo),
            password_widget: InputFieldWidget::new("Password", InputFieldDisplayType::Replace('*')),
            input_mode: InputMode::Normal,
            error_msg: None,
            graphical_environment: X::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Debug)
        // - and per-module overrides
        .level_for("hyper", log::LevelFilter::Info)
        // Output to stdout, files, and other Dispatch configurations
        .chain(fern::log_file("/tmp/lemurs.log")?)
        // Apply globally
        .apply()?;

    info!("Lemurs booting up");

    // de-hardcode 2
    if chvt::chvt(2).is_err() {
        error!("Couldn't change tty");
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    info!("UI booted up");

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    info!("Finished running UI");

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    info!("Reset terminal environment");

    if let Err(err) = res {
        println!("{:?}", err)
    }

    // TODO: Listen to signals

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Enter => {
                        app.input_mode = InputMode::WindowManager;
                    }
                    KeyCode::Char(x) if x == 'q' => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::WindowManager => match key.code {
                    KeyCode::Enter | KeyCode::Tab | KeyCode::Down => {
                        app.input_mode = InputMode::Username;
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    key_code => app.window_manager_widget.key_press(key_code),
                },
                InputMode::Username => match key.code {
                    KeyCode::Enter | KeyCode::Down => {
                        app.input_mode = InputMode::Password;
                    }
                    KeyCode::Tab => {
                        if key.modifiers == KeyModifiers::SHIFT {
                            app.input_mode = InputMode::WindowManager;
                        } else {
                            app.input_mode = InputMode::Password;
                        }
                    }
                    KeyCode::Up => {
                        app.input_mode = InputMode::WindowManager;
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    key_code => app.username_widget.key_press(key_code),
                },
                InputMode::Password => match key.code {
                    KeyCode::Enter => match login(&mut app) {
                        Err(err) => app.error_msg = Some(err),
                        _ => {}
                    },
                    KeyCode::Tab => {
                        if key.modifiers == KeyModifiers::SHIFT {
                            app.input_mode = InputMode::Username;
                        }
                    }
                    KeyCode::Up => {
                        app.input_mode = InputMode::Username;
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    key_code => app.password_widget.key_press(key_code),
                },
            }
       }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .horizontal_margin(2)
        .vertical_margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(f.size());

    app.window_manager_widget.render(
        f,
        chunks[2],
        matches!(app.input_mode, InputMode::WindowManager),
    );

    app.username_widget
        .render(f, chunks[4], matches!(app.input_mode, InputMode::Username));

    app.password_widget
        .render(f, chunks[6], matches!(app.input_mode, InputMode::Password));

    if let Some(error_msg) = &app.error_msg {
        use AuthError::*;

        let error_widget = Paragraph::new(match error_msg {
            PamError(_) => "Authentication failed",
        })
        .style(Style::default().fg(Color::Red));

        f.render_widget(error_widget, chunks[8]);
    }
}

use crate::pam::{open_session, PamError};

enum AuthError {
    PamError(PamError),
}

fn login(app: &mut App) -> Result<(), AuthError> {
    // if (!testing) {
    // signal(SIGSEGV, sig_handler);
    // signal(SIGTRAP, sig_handler);
    // }

    info!("Login attempt");

    let username = app.username_widget.get_content();
    let password = app.password_widget.get_content();
    let initrc_path = app
        .window_manager_widget
        .selected()
        .map(|selected| selected.initrc_path.clone())
        .unwrap();

    info!(
        "Gotten information. Username: '{}', Initrc_path: '{}'",
        username,
        initrc_path.to_str().unwrap_or("Not Found")
    );

    let (authenticator, passwd_entry, groups) =
        open_session(username, password).map_err(|e| AuthError::PamError(e))?;
   
    info!("Opened session");
    info!("Booting X Server");

    app.graphical_environment.start(&passwd_entry).unwrap(); // TODO: Remove unwrap

    info!("X Server started");
    info!("Booting Desktop");

    app.graphical_environment
        .desktop(initrc_path, &passwd_entry, &groups);

    info!("Desktop shutdown");

    app.graphical_environment.stop();

    info!("X server shut down");

    // Logout
    drop(authenticator);

    Ok(())
}
