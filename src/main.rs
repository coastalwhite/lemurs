use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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

mod window_manager_selector;
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
    username: String,

    /// Current value of the Password
    password: String,

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
            username: String::new(),
            password: String::new(),
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
                    KeyCode::Enter | KeyCode::Tab => {
                        app.input_mode = InputMode::Username;
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Left => {
                        app.window_manager_widget.left();
                    }
                    KeyCode::Right => {
                        app.window_manager_widget.right();
                    }
                    _ => {}
                },
                InputMode::Username => match key.code {
                    KeyCode::Enter | KeyCode::Tab => {
                        app.input_mode = InputMode::Password;
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.username.push(c);
                    }
                    _ => {}
                },
                InputMode::Password => match key.code {
                    KeyCode::Enter | KeyCode::Tab => {
                        todo!()
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.password.push(c);
                    }
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

    let username = Paragraph::new(app.username.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::WindowManager => Style::default(),
            InputMode::Username => Style::default().fg(Color::Yellow),
            InputMode::Password => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(username, chunks[4]);

    let password = Paragraph::new(app.password.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::WindowManager => Style::default(),
            InputMode::Username => Style::default(),
            InputMode::Password => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Password"));
    f.render_widget(password, chunks[6]);

    match app.input_mode {
        InputMode::Normal | InputMode::WindowManager => {}
        InputMode::Username => f.set_cursor(
            chunks[4].x + app.username.width() as u16 + 1,
            chunks[4].y + 1,
        ),
        InputMode::Password => f.set_cursor(
            chunks[6].x + app.password.width() as u16 + 1,
            chunks[6].y + 1,
        ),
    }
}
