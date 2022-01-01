use log::{error, info};
use std::error::Error;

mod graphical_environments;
mod initrcs;
mod pam;
mod ui;

use graphical_environments::X;
use ui::{run_app, App};

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

    // Start UI on a seperate thread
    let mut terminal = ui::start()?;

    run_app(&mut terminal, App::new())?;

    ui::stop(terminal)?;

    info!("Finished running UI");


    // TODO: Listen to signals

    Ok(())
}
