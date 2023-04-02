use libc::{signal, SIGUSR1, SIG_DFL, SIG_IGN};
use rand::Rng;

use once_cell::sync::Lazy;

use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fs::remove_file;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::{thread, time};

use std::path::{Path, PathBuf};

use log::{error, info};

use crate::auth::AuthUserInfo;
use crate::post_login::output_command_to_log;
use crate::LemursConfig;
use env_container::EnvironmentContainer;

const XSTART_CHECK_INTERVAL_MILLIS: u64 = 100;

#[derive(Debug, Clone)]
pub enum XSetupError {
    DisplayEnvVar,
    VTNREnvVar,
    FillingXAuth,
    InvalidUTF8Path,
    XServerStart,
    XServerTimeout,
    XServerPrematureExit,
}

impl Display for XSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayEnvVar => f.write_str("`DISPLAY` is not set"),
            Self::VTNREnvVar => f.write_str("`XDG_VTNR` is not set"),
            Self::FillingXAuth => f.write_str("Failed to fill `.Xauthority` file"),
            Self::InvalidUTF8Path => f.write_str("Path that is given is not valid UTF8"),
            Self::XServerStart => f.write_str("Failed to start X server binary"),
            Self::XServerTimeout => f.write_str("Timeout while waiting for X server to start"),
            Self::XServerPrematureExit => {
                f.write_str("X server exited before it signaled to accept connections")
            }
        }
    }
}

impl Error for XSetupError {}

fn mcookie() -> String {
    // TODO: Verify that this is actually safe. Maybe just use the mcookie binary?? Is that always
    // available?
    let mut rng = rand::thread_rng();
    let cookie: u128 = rng.gen();
    format!("{cookie:032x}")
}

static X_HAS_STARTED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[allow(dead_code)]
fn handle_sigusr1(_: i32) {
    X_HAS_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);

    unsafe {
        signal(SIGUSR1, handle_sigusr1 as usize);
    }
}

pub fn setup_x(
    process_env: &mut EnvironmentContainer,
    user_info: &AuthUserInfo,
    config: &LemursConfig,
) -> Result<Child, XSetupError> {
    use std::os::unix::process::CommandExt;

    info!("Start setup of X server");

    let display_value = env::var("DISPLAY").map_err(|_| XSetupError::DisplayEnvVar)?;
    let vtnr_value = env::var("XDG_VTNR").map_err(|_| XSetupError::VTNREnvVar)?;

    // Setup xauth
    let xauth_dir =
        PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| user_info.dir.to_string()));
    let xauth_path = xauth_dir.join(".Xauthority");

    info!(
        "Filling `.Xauthority` file at `{xauth_path}`",
        xauth_path = xauth_path.display()
    );

    // Make sure that we are generating a new file. This is necessary since sometimes, there may be
    // a `root` permission `.Xauthority` file there.
    let _ = remove_file(xauth_path.clone());

    Command::new(super::SYSTEM_SHELL)
        .arg("-c")
        .arg(format!(
            "/usr/bin/xauth add {} . {}",
            display_value,
            mcookie()
        ))
        .uid(user_info.uid)
        .gid(user_info.gid)
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .status()
        .map_err(|err| {
            error!(
                "Failed to fill Xauthority file with `xauth`. Reason: {}",
                err
            );
            XSetupError::FillingXAuth
        })?;

    let xauth_path = xauth_path.to_str().ok_or(XSetupError::InvalidUTF8Path)?;
    process_env.set("XAUTHORITY", xauth_path);

    let doubledigit_vtnr = if vtnr_value.len() == 1 {
        format!("0{vtnr_value}")
    } else {
        vtnr_value
    };

    // Here we explicitely ignore the first USR defined signal. Xorg looks at whether this signal
    // is ignored or not. If it is ignored, it will send that signal to the parent when it ready to
    // receive connections. This is also how xinit does it.
    //
    // After we spawn the Xorg process, we need to make sure to quickly re-enable this signal as we
    // need to listen to the signal by Xorg.
    unsafe {
        libc::signal(SIGUSR1, SIG_IGN);
    }

    let mut child = Command::new(super::SYSTEM_SHELL);

    let mut child = if config.do_log {
        info!(
            "Setup XServer to log `stdout` and `stderr` to '{log_path}'",
            log_path = config.xserver_log_path
        );
        output_command_to_log(child, Path::new(&config.xserver_log_path))
    } else {
        child.stdout(Stdio::null()).stderr(Stdio::null());
        child
    };

    let mut child = child
        .arg("-c")
        .arg(format!("/usr/bin/X {display_value} vt{doubledigit_vtnr}"))
        .spawn()
        .map_err(|err| {
            error!("Failed to start X server. Reason: {}", err);
            XSetupError::XServerStart
        })?;

    // See note above
    unsafe {
        libc::signal(SIGUSR1, SIG_DFL);
        signal(SIGUSR1, handle_sigusr1 as usize);
    }

    // Wait for XServer to boot-up
    let start_time = time::SystemTime::now();
    loop {
        if config.xserver_timeout_secs == 0
            || start_time
                .elapsed()
                .map_or(false, |t| t.as_secs() > config.xserver_timeout_secs.into())
        {
            break;
        }

        // This will be set by the `handle_sigusr1` signal handler.
        if X_HAS_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        if let Some(status) = child.try_wait().unwrap_or(None) {
            error!("X server died before signaling it was ready to received connections. Status code: {status}.");

            return Err(XSetupError::XServerPrematureExit);
        }

        thread::sleep(time::Duration::from_millis(XSTART_CHECK_INTERVAL_MILLIS));
    }

    // If the value is still `false`, this means we have time-ed out and Xorg is not running.
    if !X_HAS_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
        child.kill().unwrap_or_else(|err| {
            error!("Failed to kill Xorg after it timed out. Reason: {err}");
        });
        return Err(XSetupError::XServerTimeout);
    }

    if let Ok(x_server_start_time) = start_time.elapsed() {
        info!(
            "It took X server {start_ms}ms to start",
            start_ms = x_server_start_time.as_millis()
        );
    }

    X_HAS_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);

    info!("X server is running");

    Ok(child)
}
