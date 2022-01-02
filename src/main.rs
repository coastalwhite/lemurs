use std::error::Error;

use log::{error, info};
use clap::{arg, App as ClapApp};

mod graphical_environments;
mod initrcs;
mod pam;
mod ui;
mod config;

use graphical_environments::X;
use ui::{run_app, App};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = ClapApp::new("Lemurs")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(--preview))
        .get_matches();

    let mut config = config::Config::default();
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
        .chain(fern::log_file("/tmp/lemurs.log")?)
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
