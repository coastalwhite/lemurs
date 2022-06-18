use std::error::Error;
use std::process;

use clap::{arg, App as ClapApp};
use log::{error, info, warn};

use config::Config;

mod config;
mod environment;
mod graphical_environments;
mod info_caching;
mod initrcs;
mod pam;
mod ui;

const DEFAULT_CONFIG_PATH: &str = "/etc/lemurs/config.toml";
const PREVIEW_LOG_PATH: &str = "lemurs.log";
const DEFAULT_LOG_PATH: &str = "/var/log/lemurs.log";

fn merge_in_configuration(config: &mut Config, config_path: Option<&str>) {
    let load_config_path = config_path.unwrap_or(DEFAULT_CONFIG_PATH);

    match config::PartialConfig::from_file(load_config_path) {
        Ok(partial_config) => {
            info!(
                "Successfully loaded configuration file from '{}'",
                load_config_path
            );
            config.merge_in_partial(partial_config)
        }
        Err(err) => {
            // If we have given it a specific config path, it should crash if this file cannot be
            // loaded. If it is the default config location just put a warning in the logs.
            if let Some(config_path) = config_path {
                eprintln!(
                    "The config file '{}' cannot be loaded.\nReason: {}",
                    config_path, err
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

fn setup_logger(is_preview: bool) {
    let log_path = if is_preview {
        PREVIEW_LOG_PATH
    } else {
        DEFAULT_LOG_PATH
    };

    let log_file = fern::log_file(log_path).unwrap_or_else(|err| {
        eprintln!(
            "Failed to open log file: '{}'. Check that the path is valid or activate `--no-log`. Reason: {}",
            log_path, err
        );
        process::exit(1);
    });

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("hyper", log::LevelFilter::Info)
        .chain(log_file)
        .apply()
        .unwrap_or_else(|err| {
            eprintln!(
                "Failed to setup logger. Fix the error or activate `--no-log`. Reason: {}",
                err
            );
            process::exit(1);
        });
}

use ui::{run_app, App};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = ClapApp::new("Lemurs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(--preview))
        .arg(arg!(--nolog))
        .arg(arg!(-c --config [FILE] "a file to replace the default configuration"))
        .get_matches();

    let no_log = matches.is_present("nolog");
    let preview = matches.is_present("preview");

    // Setup the logger
    if !no_log {
        setup_logger(preview);
    }

    info!("Lemurs logger is running");

    // Load and setup configuration
    let mut config = Config::default();
    merge_in_configuration(&mut config, matches.value_of("config"));

    if !preview {
        // Switch to the proper tty
        info!("Switching to tty {}", config.tty);

        chvt::chvt(config.tty.into()).unwrap_or_else(|err| {
            error!("Failed to switch tty {}. Reason: {}", config.tty, err);
        });
    }

    // Start application
    let mut terminal = ui::start()?;
    run_app(&mut terminal, App::new(config, preview))?;
    ui::stop(terminal)?;

    info!("Lemurs is booting down");

    Ok(())
}
