use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

mod input_field;
mod window_manager_selector;
use input_field::{InputFieldDisplayType, InputFieldWidget};
use window_manager_selector::{WindowManager, WindowManagerSelectorWidget};

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
}

impl Default for App {
    fn default() -> App {
        App {
            window_manager_widget: WindowManagerSelectorWidget::new(vec![
                WindowManager::new("bspwm", "sxhkd & ; exec bspwm"),
                WindowManager::new("i3", "/usr/bin/i3"),
                WindowManager::new("awesome", "/usr/bin/awesome"),
            ]),
            username_widget: InputFieldWidget::new("Username", InputFieldDisplayType::Echo),
            password_widget: InputFieldWidget::new("Password", InputFieldDisplayType::Replace('*')),
            input_mode: InputMode::Normal,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

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
                    KeyCode::Enter => {
                        todo!()
                    }
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
                    _ => {}
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
}
