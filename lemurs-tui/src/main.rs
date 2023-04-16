use std::error::Error;
use std::fs::File;
use std::io;
use std::path::Path;
use std::process;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, info, warn};
use tui::backend::CrosstermBackend;
use tui::Terminal;

mod cli;
mod config;
mod info_caching;
mod ui;

use crate::cli::{Cli, Commands};
use config::Config;

const DEFAULT_CONFIG_PATH: &str = "/etc/lemurs/config.toml";
const PREVIEW_LOG_PATH: &str = "lemurs.log";

fn merge_in_configuration(config: &mut Config, config_path: Option<&Path>) {
    let load_config_path = config_path.unwrap_or_else(|| Path::new(DEFAULT_CONFIG_PATH));

    match config::PartialConfig::from_file(load_config_path) {
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
            if let Some(config_path) = config_path {
                eprintln!(
                    "The config file '{}' cannot be loaded.\nReason: {}",
                    config_path.display(),
                    err
                );
                process::exit(1);
            } else {
                warn!(
                    "No configuration file loaded from the expected location ({}). Reason: {}",
                    DEFAULT_CONFIG_PATH, err
                );
            }
        }
    }
}

fn setup_logger(log_path: &str) {
    let log_file = Box::new(File::create(log_path).unwrap_or_else(|_| {
        eprintln!("Failed to open log file: '{log_path}'");
        std::process::exit(1);
    }));

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(log_file))
        .init();
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse().unwrap_or_else(|err| {
        eprintln!("{err}\n");
        cli::usage();
        std::process::exit(2);
    });

    // Load and setup configuration
    let mut config = Config::default();
    merge_in_configuration(&mut config, cli.config.as_deref());

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Envs => {
                let envs = lemurs_core::post_login::get_envs(
                    config.environment_switcher.include_tty_shell,
                );

                for (env_name, _) in envs.into_iter() {
                    println!("{env_name}");
                }
            }
            Commands::Cache => {
                let cached_info = info_caching::get_cached_information();

                let environment = cached_info.environment().unwrap_or("No cached value");
                let username = cached_info.username().unwrap_or("No cached value");

                println!(
                    "Information currently cached within '{}'\n",
                    info_caching::CACHE_PATH
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
            eprintln!("Lemurs cannot be ran without `--preview` within an existing session. Namely, `XDG_SESSION_TYPE` is set.");
            error!("Lemurs cannot be started when within an existing session. Namely, `XDG_SESSION_TYPE` is set.");
            std::process::exit(1);
        }

        let uid = users::get_current_uid();
        if users::get_current_uid() != 0 {
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
    }

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
