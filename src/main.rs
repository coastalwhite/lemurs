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
const DEFAULT_LOG_PATH: &str = "/var/log/lemurs.log";

pub fn merge_in_configuration(config: &mut Config, config_path: Option<&str>) {
    match config::PartialConfig::from_file(config_path.unwrap_or(DEFAULT_CONFIG_PATH)) {
        Ok(partial_config) => config.merge_in_partial(partial_config),
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
                    "No configuration file loaded from the expected location ({})",
                    DEFAULT_CONFIG_PATH
                );
            }
        }
    }
}

use graphical_environments::X;
use ui::{run_app, App};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = ClapApp::new("Lemurs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(--preview))
        .arg(arg!(-c --config [FILE] "a file to replace the default configuration"))
        .get_matches();

    let preview = matches.is_present("preview");
    let mut config = Config::default();
    merge_in_configuration(&mut config, matches.value_of("config"));

    info!("Started");

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
        // As of now just log to the /tmp/lemurs.log
        .chain(fern::log_file(if preview {
            "out.log"
        } else {
            DEFAULT_LOG_PATH
        })?)
        .apply()?;

    info!("Initiated logger");

    if !preview {
        // Switch to the proper tty
        if chvt::chvt(config.tty.into()).is_err() {
            error!("Failed to switch TTY");
        };
        info!("Successfully switched TTY");
    }

    // Start application
    let mut terminal = ui::start()?;
    run_app(&mut terminal, App::new(config, preview))?;
    ui::stop(terminal)?;

    info!("Booting down");

    // TODO: Listen to signals

    Ok(())
}
