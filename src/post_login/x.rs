use nix::unistd::Gid;
use rand::Rng;
use std::env;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::{thread, time};
use users::get_user_groups;

use std::path::PathBuf;

use log::{error, info};

use crate::auth::AuthUserInfo;

const DISPLAY: &str = ":1";
const VIRTUAL_TERMINAL: &str = "vt01";

const SYSTEM_SHELL: &str = "/bin/sh";

const XSTART_TIMEOUT_SECS: u64 = 20;
const XSTART_CHECK_INTERVAL_MILLIS: u64 = 100;

pub enum XSetupError {
    FillingXAuth,
    XServerStart,
}

pub enum XStartEnvError {
    StartingEnvironment,
}

fn mcookie() -> String {
    // TODO: Verify that this is actually safe. Maybe just use the mcookie binary?? Is that always
    // available?
    let mut rng = rand::thread_rng();
    let cookie: u128 = rng.gen();
    format!("{:032x}", cookie)
}

pub fn setup_x(user_info: &AuthUserInfo) -> Result<Child, XSetupError> {
    info!("Start setup of X");

    // Setup xauth
    let xauth_dir =
        PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| user_info.dir.to_string()));
    let xauth_path = xauth_dir.join(".Xauthority");
    env::set_var("XAUTHORITY", xauth_path);
    env::set_var("DISPLAY", DISPLAY);

    info!("Filling Xauthority file");
    Command::new(SYSTEM_SHELL)
        .arg("-c")
        .arg(format!("/usr/bin/xauth add {} . {}", DISPLAY, mcookie()))
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .status()
        .map_err(|err| {
            error!("Filling xauth file failed. Reason: {}", err);
            XSetupError::FillingXAuth
        })?;

    info!("Run X server");
    let child = Command::new(SYSTEM_SHELL)
        .arg("-c")
        .arg(format!("/usr/bin/X {} {}", DISPLAY, VIRTUAL_TERMINAL))
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .spawn()
        .map_err(|err| {
            error!("Starting X server failed. Reason: {}", err);
            XSetupError::XServerStart
        })?;

    // Wait for XServer to boot-up
    let start_time = time::SystemTime::now();
    loop {
        // Timeout
        if match start_time.elapsed() {
            Ok(dur) => dur.as_secs() >= XSTART_TIMEOUT_SECS,
            Err(_) => {
                error!("Failed to resolve elapsed time");
                std::process::exit(1);
            }
        } {
            return Err(XSetupError::XServerStart);
        }

        match Command::new(SYSTEM_SHELL)
            .arg("-c")
            .arg("timeout 1s /usr/bin/xset q")
            .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
            .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
            .status()
        {
            Ok(status) => {
                if status.success() {
                    break;
                }
            }
            Err(_) => {
                error!("Failed to run xset to check X server status");
                return Err(XSetupError::XServerStart);
            }
        }

        thread::sleep(time::Duration::from_millis(XSTART_CHECK_INTERVAL_MILLIS));
    }
    info!("X server is running");

    Ok(child)
}

pub fn start_env(user_info: &AuthUserInfo, script_path: &str) -> Result<Child, XStartEnvError> {
    let uid = user_info.uid;
    let gid = user_info.gid;
    let groups: Vec<Gid> = get_user_groups(&user_info.name, gid)
        .unwrap()
        .iter()
        .map(|group| Gid::from_raw(group.gid()))
        .collect();

    info!("Starting specified environment");
    let mut cmd = Command::new(SYSTEM_SHELL);
    let cmd = cmd
        .arg("-c")
        .arg(format!("{} {}", "/etc/lemurs/xsetup.sh", script_path))
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .uid(uid)
        .gid(gid);
    let cmd =
        unsafe { cmd.pre_exec(move || nix::unistd::setgroups(&groups).map_err(|err| err.into())) };

    let child = cmd.spawn().map_err(|err| {
        error!("Failed to start specified environment. Reason: {}", err);
        XStartEnvError::StartingEnvironment
    })?;

    info!("Started specified environment");

    Ok(child)
}
