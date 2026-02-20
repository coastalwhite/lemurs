use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::{error::Error, path::Path};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, warn};
use nix::fcntl::FdFlag;
use nix::fcntl::{fcntl, FcntlArg};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod auth;
mod chvt;
mod cli;
mod config;
mod env_container;
mod info_caching;
mod post_login;
mod ui;

use auth::try_auth;
use config::Config;
use post_login::{EnvironmentStartError, PostLoginEnvironment};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::auth::utmpx::UtmpxSession;
use crate::post_login::wait_with_log::LemursChild;
use crate::{
    auth::utmpx::add_utmpx_entry,
    cli::{Cli, Commands},
};

use self::{
    auth::AuthenticationError,
    env_container::EnvironmentContainer,
    post_login::env_variables::{
        remove_xdg, set_basic_variables, set_display, set_seat_vars, set_session_params,
        set_session_vars, set_xdg_common_paths,
    },
};

const DEFAULT_VARIABLES_PATH: &str = "/etc/lemurs/variables.toml";
const DEFAULT_CONFIG_PATH: &str = "/etc/lemurs/config.toml";
const PREVIEW_LOG_PATH: &str = "lemurs.log";

fn merge_in_configuration(config: &mut Config, cli: &Cli) {
    let load_variables_path = cli
        .variables
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_VARIABLES_PATH));

    if let Some(initial_path) = &cli.initial_path {
        config.initial_path = initial_path.clone();
    }

    let variables = match config::Variables::from_file(load_variables_path) {
        Ok(variables) => {
            info!(
                "Successfully loaded variables file from '{}'",
                load_variables_path.display()
            );

            Some(variables)
        }
        Err(err) => {
            // If we have given it a specific config path, it should crash if this file cannot be
            // loaded. If it is the default config location just put a warning in the logs.
            if let Some(variables_path) = cli.variables.as_ref() {
                eprintln!(
                    "The variables file '{}' cannot be loaded.\nReason: {}",
                    variables_path.display(),
                    err
                );
                std::process::exit(1);
            } else {
                info!(
                    "No variables file loaded from the default location ({}). Reason: {}",
                    DEFAULT_CONFIG_PATH, err
                );
            }

            None
        }
    };

    let load_config_path = cli
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_PATH));

    match config::PartialConfig::from_file(load_config_path, variables.as_ref()) {
        Ok(partial_config) => {
            info!(
                "Successfully loaded configuration file from '{}'",
                load_config_path.display()
            );
            config.merge_in_partial(partial_config)
        }
        Err(err) => {
            // If we have given it a specific config path, it should crash if this file cannot be
            // loaded. If it is the default config location just put a warning in the logs.
            if let Some(config_path) = cli.config.as_ref() {
                eprintln!(
                    "The config file '{}' cannot be loaded.\nReason: {}",
                    config_path.display(),
                    err
                );
                std::process::exit(1);
            } else {
                warn!(
                    "No configuration file loaded from the expected location ({}). Reason: {}",
                    DEFAULT_CONFIG_PATH, err
                );
            }
        }
    }

    if let Some(xsessions) = cli.xsessions.as_ref() {
        config.x11.xsessions_path = xsessions.display().to_string();
    }

    if let Some(wlsessions) = cli.wlsessions.as_ref() {
        config.wayland.wayland_sessions_path = wlsessions.display().to_string();
    }
}

pub fn initialize_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen).unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();

        original_hook(panic_info);
    }));
}

fn setup_logger(log_path: &str) {
    let log_file = Box::new(File::create(log_path).unwrap_or_else(|_| {
        eprintln!("Failed to open log file: '{log_path}'");
        std::process::exit(1);
    }));

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(log_file))
        .format_timestamp_secs()
        .init();
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse().unwrap_or_else(|err| {
        eprintln!("{err}\n");
        cli::usage();
        std::process::exit(2);
    });

    let mut config = Config::default();
    merge_in_configuration(&mut config, &cli);

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Envs => {
                let envs = post_login::get_envs(&config);

                for (env_name, _) in envs.into_iter() {
                    println!("{env_name}");
                }
            }
            Commands::Cache => {
                let cached_info = info_caching::get_cached_information(&config);

                let environment = cached_info.environment().unwrap_or("No cached value");
                let username = cached_info.username().unwrap_or("No cached value");

                println!(
                    "Information currently cached within '{}'\n",
                    config.cache_path
                );

                println!("environment: '{environment}'");
                println!("username: '{username}'");
            }
            Commands::Help => {
                cli::usage();
            }
            Commands::Version => {
                println!("{}", env!("CARGO_PKG_VERSION"));
            }
            Commands::Session { fd } => start_session(fd),
        }

        return Ok(());
    }

    // Setup the logger
    if !cli.no_log {
        setup_logger(if cli.preview {
            PREVIEW_LOG_PATH
        } else {
            &config.main_log_path
        });
        info!("Main lemurs logger is running");
    } else {
        config.do_log = false;
    }

    if !cli.preview {
        if std::env::var("XDG_SESSION_TYPE").is_ok() {
            eprintln!(
                "Lemurs cannot be ran without `--preview` within an existing session. Namely, `XDG_SESSION_TYPE` is set."
            );
            error!(
                "Lemurs cannot be started when within an existing session. Namely, `XDG_SESSION_TYPE` is set."
            );
            std::process::exit(1);
        }

        let uid = uzers::get_current_uid();
        if uzers::get_current_uid() != 0 {
            eprintln!("Lemurs needs to be ran as root. Found user id '{uid}'");
            error!("Lemurs not ran as root. Found user id '{uid}'");
            std::process::exit(1);
        }

        if let Some(tty) = cli.tty {
            info!("Overwritten the tty to '{tty}' with the --tty flag");
            config.tty = tty;
        }

        // Switch to the proper tty
        info!("Switching to tty {}", config.tty);

        unsafe { chvt::chvt(config.tty.into()) }.unwrap_or_else(|err| {
            error!("Failed to switch tty {}. Reason: {err}", config.tty);
        });
    }

    initialize_panic_handler();

    // Start application
    let mut terminal = tui_enable()?;
    let login_form = ui::LoginForm::new(config, cli.preview);
    login_form.run(&mut terminal)?;
    tui_disable(terminal)?;

    info!("Lemurs is booting down");

    Ok(())
}

pub fn tui_enable() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    info!("UI booted up");

    Ok(terminal)
}

pub fn tui_disable(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    info!("Reset terminal environment");

    Ok(())
}

struct Hooks<'a> {
    pre_validate: Option<&'a dyn Fn()>,
    pre_auth: Option<&'a dyn Fn()>,
    pre_environment: Option<&'a dyn Fn()>,
    pre_wait: Option<&'a dyn Fn()>,
    pre_return: Option<&'a dyn Fn()>,
}

pub enum StartSessionError {
    AuthenticationError(AuthenticationError),
    EnvironmentStartError(EnvironmentStartError),
    ChildIo(io::Error),
}

impl From<EnvironmentStartError> for StartSessionError {
    fn from(value: EnvironmentStartError) -> Self {
        Self::EnvironmentStartError(value)
    }
}

impl From<AuthenticationError> for StartSessionError {
    fn from(value: AuthenticationError) -> Self {
        Self::AuthenticationError(value)
    }
}
impl From<io::Error> for StartSessionError {
    fn from(value: io::Error) -> Self {
        Self::ChildIo(value)
    }
}

fn webext_write<T: Serialize, W: Write>(w: &mut W, t: T) -> io::Result<()> {
    let serialized = serde_json::to_vec(&t).map_err(io::Error::other)?;
    let len = (serialized.len() as u32).to_be_bytes();
    w.write_all(&len)?;
    w.write_all(&serialized)?;
    Ok(())
}

fn webext_read<T: DeserializeOwned, R: Read>(r: &mut R) -> io::Result<T> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > 65535 {
        // Realistically, none of the subprocess messages should exceed this size
        // Subprocess invokes pam, and theoretically, rogue pam module might output something to stdout (it shouldn't, but it might)
        // and it would be very bad it its response will cause us to allocate too much
        return Err(io::Error::other("corrupted response: len"));
    }

    let mut serialized = vec![0u8; len];
    r.read_exact(&mut serialized)?;

    let t: T = serde_json::from_slice(&serialized).map_err(io::Error::other)?;

    Ok(t)
}

fn webext_request<O: DeserializeOwned, I: Serialize, W: Write + Read>(
    w: &mut W,
    i: I,
) -> io::Result<O> {
    webext_write(w, i)?;
    webext_read(w)
}

#[derive(Serialize, Deserialize)]
enum ChildRequest {
    Config,
    PostLoginEnv,
    Username,
    Password,
    PreAuth,
    PreEnvironment,
    PreWait,

    AuthenticationError(AuthenticationError),
    EnvironmentStartError(EnvironmentStartError),
    ChildIo(String),
}

fn start_session_child(
    username: &str,
    password: &str,
    post_login_env: &PostLoginEnvironment,
    config: &Config,
    hooks: &Hooks<'_>,
) -> Result<(), StartSessionError> {
    let exe = std::env::current_exe()?;

    if let Some(pre_validate_hook) = hooks.pre_validate {
        pre_validate_hook();
    }

    let (mut ipc, ipc_child) = UnixStream::pair()?;
    // Rust stdlib uses CLOEXEC for UnixStream::pair, meaning it will die when passed to the child process
    // CLOEXEC will be returned by the childprocess, so the socket will be closed when the childprocess switches to the underlying session command
    fcntl(ipc_child.as_raw_fd(), FcntlArg::F_SETFD(FdFlag::empty()))
        .map_err(|e| io::Error::other(format!("fnctl: {e}")))?;

    let mut cmd = Command::new(exe);
    cmd.arg("session").arg(ipc_child.as_raw_fd().to_string());

    let log_path = config.do_log.then_some(Path::new(&config.client_log_path));
    let mut child = LemursChild::spawn(cmd, log_path)?;

    drop(ipc_child);

    let mut _utmpx_session = None::<UtmpxSession>;
    loop {
        let r: ChildRequest = match webext_read(&mut ipc) {
            Ok(r) => r,
            // Reading len would fail with UnexpectedEof when the child socket is closed (session grandchild is started)
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
        match r {
            ChildRequest::Config => {
                webext_write(&mut ipc, config)?;
            }
            ChildRequest::PostLoginEnv => {
                webext_write(&mut ipc, post_login_env)?;
            }
            ChildRequest::Username => {
                webext_write(&mut ipc, username)?;
            }
            ChildRequest::Password => {
                webext_write(&mut ipc, password)?;
            }
            ChildRequest::PreAuth => {
                if let Some(pre_auth_hook) = hooks.pre_auth {
                    pre_auth_hook();
                }
            }
            ChildRequest::PreEnvironment => {
                if let Some(pre_environment_hook) = hooks.pre_environment {
                    pre_environment_hook();
                }
            }
            ChildRequest::PreWait => {
                _utmpx_session = Some(add_utmpx_entry(username, config.tty, child.id()));

                if let Some(pre_wait_hook) = hooks.pre_wait {
                    pre_wait_hook();
                }
            }
            ChildRequest::AuthenticationError(authentication_error) => {
                return Err(StartSessionError::AuthenticationError(authentication_error))
            }
            ChildRequest::EnvironmentStartError(environment_start_error) => {
                return Err(StartSessionError::EnvironmentStartError(
                    environment_start_error,
                ))
            }
            ChildRequest::ChildIo(e) => {
                return Err(StartSessionError::ChildIo(io::Error::other(e)))
            }
        }
    }

    let status = child.wait()?;

    if let Some(pre_return_hook) = hooks.pre_return {
        pre_return_hook();
    }

    match status.code() {
        Some(0) => Ok(()),
        v => Err(StartSessionError::ChildIo(io::Error::other(format!(
            "unexpected exit code: {v:?}"
        )))),
    }
}

fn start_session(fd: RawFd) {
    // SAFETY: We don't care for IO safety here,
    // if the parent has given us the wrong fd - we can just die
    let ipc = &mut unsafe { UnixStream::from_raw_fd(fd) };
    fcntl(ipc.as_raw_fd(), FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).expect("cloexec flag return");
    let Err(e) = start_session_inner(ipc) else {
        return;
    };
    match e {
        StartSessionError::AuthenticationError(authentication_error) => {
            let _ = webext_write(ipc, ChildRequest::AuthenticationError(authentication_error));
        }
        StartSessionError::EnvironmentStartError(environment_start_error) => {
            let _ = webext_write(
                ipc,
                ChildRequest::EnvironmentStartError(environment_start_error),
            );
        }
        StartSessionError::ChildIo(error) => {
            let _ = webext_write(ipc, ChildRequest::ChildIo(error.to_string()));
        }
    }
}

fn start_session_inner(ipc: &mut UnixStream) -> Result<(), StartSessionError> {
    let username = &webext_request::<String, _, _>(ipc, ChildRequest::Username)?;
    let password = &webext_request::<String, _, _>(ipc, ChildRequest::Password)?;

    let config = &webext_request::<Config, _, _>(ipc, ChildRequest::Config)?;

    let post_login_env =
        &webext_request::<PostLoginEnvironment, _, _>(ipc, ChildRequest::PostLoginEnv)?;

    info!(
        "Starting new session for '{}' in environment '{:?}'",
        username, post_login_env
    );

    let mut process_env = EnvironmentContainer::take_snapshot();

    if matches!(post_login_env, PostLoginEnvironment::X { .. }) {
        set_display(&config.x11.x11_display, &mut process_env);
    }
    set_session_params(&mut process_env, post_login_env);
    remove_xdg(&mut process_env);
    set_seat_vars(&mut process_env, config.tty);

    webext_write(ipc, ChildRequest::PreAuth)?;

    let auth_session = try_auth(username, password, &config.pam_service)?;

    webext_write(ipc, ChildRequest::PreEnvironment)?;

    let uid = auth_session.uid;
    let homedir = &auth_session.home_dir;
    let shell = &auth_session.shell;

    set_session_vars(&mut process_env, uid);
    set_basic_variables(
        &mut process_env,
        username,
        homedir,
        shell,
        &config.initial_path,
    );
    set_xdg_common_paths(&mut process_env, homedir);

    webext_write(ipc, ChildRequest::PreWait)?;

    post_login_env.exec(&auth_session, &mut process_env, config)?;
    unreachable!("exec() should not return")
}
