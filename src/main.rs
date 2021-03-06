use std::error::Error;

use clap::{arg, App as ClapApp};
use log::{error, info};

mod config;
mod graphical_environments;
mod initrcs;
mod pam;
mod ui;

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

    let mut config = if let Some(config_path) = matches.value_of("config") {
        config::Config::from_file(config_path).expect("Unable to open given configuration file.")
    } else {
        config::Config::default()
    };
    config.preview = matches.is_present("preview");

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
        .chain(fern::log_file(if config.preview {
            "out.log"
        } else {
            "/tmp/lemurs.log"
        })?)
        .apply()?;

    info!("Initiated logger");

    if !config.preview {
        // Switch to the proper tty
        if chvt::chvt(2).is_err() {
            error!("Failed to switch TTY");
        };
        info!("Successfully switched TTY");
    }

    // Start application
    let mut terminal = ui::start()?;
    run_app(&mut terminal, App::new(config))?;
    ui::stop(terminal)?;

    info!("Booting down");

    // TODO: Listen to signals

    Ok(())
}
