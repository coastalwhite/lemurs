use rand::Rng;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::{env, fs};
use std::{thread, time};

use std::path::PathBuf;

use log::{error, info, warn};

use crate::auth::SessionUser;

use super::{EnvironmentContext, SessionInitializer};

const SERVER_QUERY_NUM_OF_TRIES: usize = 10;
const SERVER_QUERY_TIMEOUT: Duration = Duration::from_millis(1000);
const TIMEOUT_CHECK_INTERVAL: Duration = Duration::from_millis(100);

const X11_SESSIONS_DIR: &str = "/etc/lemurs/wms";

pub struct X11StartContext<'a> {
    system_shell: &'a str,
    display: &'a str,
    virtual_terminal: &'a str,
    x_bin_path: &'a str,
}

impl<'a> From<&EnvironmentContext<'a>> for X11StartContext<'a> {
    fn from(context: &EnvironmentContext<'a>) -> Self {
        let EnvironmentContext {
            system_shell,
            display,
            virtual_terminal,
            x_bin_path,
            ..
        } = context;

        Self {
            system_shell,
            display,
            virtual_terminal,
            x_bin_path,
        }
    }
}

fn mcookie() -> String {
    // TODO: Verify that this is actually safe. Maybe just use the mcookie binary?? Is that always
    // available?
    let mut rng = rand::thread_rng();
    let cookie: u128 = rng.gen();
    format!("{:032x}", cookie)
}

pub fn setup_x_server(
    user_info: &SessionUser,
    context: &X11StartContext,
) -> Result<Child, X11StartError> {
    use std::os::unix::process::CommandExt;

    info!("Start setup of X");

    // Setup xauth
    let xauth_dir = PathBuf::from(
        env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| user_info.home_dir().to_string()),
    );
    let xauth_path = xauth_dir.join(".Xauthority");
    env::set_var("XAUTHORITY", xauth_path);
    env::set_var("DISPLAY", context.display);

    info!("Filling Xauthority file");
    Command::new(context.system_shell)
        .arg("-c")
        .arg(format!(
            "/usr/bin/xauth add {} . {}",
            context.display,
            mcookie()
        ))
        .uid(user_info.user_id().as_raw())
        .gid(user_info.group_id().as_raw())
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .status()
        .map_err(|err| {
            error!("Filling xauth file failed. Reason: {}", err);
            X11StartError::XAuthCommand
        })?;

    info!("Run X server");
    let child = Command::new(context.system_shell)
        .arg("-c")
        .arg(format!(
            "{} {} {}",
            context.x_bin_path, context.display, context.virtual_terminal
        ))
        .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
        .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
        .spawn()
        .map_err(|err| {
            error!("Starting X server failed. Reason: {}", err);
            X11StartError::XServerStart
        })?;

    // Wait for XServer to boot-up
    info!("Wait for X server to boot up");
    let mut num_tries = 0;
    loop {
        let start_time = time::SystemTime::now();
        let mut query_command = Command::new(context.system_shell)
            .arg("-c")
            .arg("/usr/bin/xset q")
            .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
            .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
            .spawn()
            .map_err(|_| {
                error!("Failed to run xset to check X server status");
                X11StartError::FailedServerStartQuery
            })?;

        // Loop until the querying command is done. This has a timeout at which point it is retried
        // with a new command.
        let option_status = loop {
            if start_time
                .elapsed()
                .map(|dur| dur < SERVER_QUERY_TIMEOUT)
                .map_err(|_| {
                    error!("Failed to resolve elapsed time");
                    X11StartError::FailedResolveElapsedTime
                })?
            {
                break None;
            }

            // Wait for a bit for the query command to finish
            thread::sleep(TIMEOUT_CHECK_INTERVAL);

            // See if the query command has finished
            match query_command.try_wait() {
                Err(_) => {
                    error!("Failed check status of query command");
                    return Err(X11StartError::FailedServerStatusCheck);
                }
                Ok(None) => continue,
                Ok(Some(status)) => break Some(status),
            };
        };

        match option_status {
            Some(status) if status.success() => break,
            Some(status) => warn!(
                "X Server query command exited with exit status '{:?}'",
                status.code()
            ),
            None => warn!("X Server query command timed out"),
        };

        // Exceeded the max number of tries
        if num_tries >= SERVER_QUERY_NUM_OF_TRIES {
            error!("Checking the X server status has exceeded its maximum number of tries.");
            return Err(X11StartError::ExceededMaxTries);
        }
        num_tries += 1;
    }
    info!("X server is running");

    Ok(child)
}

pub enum X11StartError {
    XAuthCommand,
    XServerStart,
    ServerTimeout,
    FailedServerStartQuery,
    FailedServerStatusCheck,
    FailedResolveElapsedTime,
    ExceededMaxTries,
}

impl Default for X11StartContext<'static> {
    fn default() -> Self {
        (&EnvironmentContext::default()).into()
    }
}

impl SessionInitializer {
    pub fn start_x11(
        &self,
        session_user: &SessionUser,
        context: &X11StartContext,
    ) -> Result<Command, X11StartError> {
        info!("Starting X11 session '{}'", self.name);

        // Start the X Server
        setup_x_server(session_user, context)?;

        let mut initializer = Command::new(context.system_shell);

        // Make it run the initializer
        initializer.arg("-c").arg(format!(
            "{} {}",
            "/etc/lemurs/xsetup.sh",
            self.path.display()
        ));

        Ok(initializer)
    }
}

pub fn get_envs() -> Vec<SessionInitializer> {
    let Ok(dir_entries) = fs::read_dir(X11_SESSIONS_DIR) else {
        warn!(
            "Failed to read from the x11 sessions folder '{}'",
            X11_SESSIONS_DIR
        );
        return Vec::new();
    };

    let capacity = match dir_entries.size_hint() {
        (_, Some(upperbound)) => upperbound,
        (lowerbound, _) => lowerbound,
    };
    let mut envs = Vec::with_capacity(capacity);

    for dir_entry in dir_entries {
        // Check validity of path
        let Ok(dir_entry) = dir_entry else {
            warn!("Ignored errorinous x11 path: '{}'", dir_entry.unwrap_err());
            continue;
        };

        // Check UTF-8 compatability of file_name
        let Ok(file_name) = dir_entry.file_name().into_string() else {
            warn!("Unable to convert OSString to String. Skipping x11 item");
            continue;
        };

        // Get file metadata
        let Ok(metadata) = dir_entry.metadata() else {
            warn!("Unable to fetch file metadata. Skipping x11 item");
            continue;
        };

        // Make sure the file is executable
        if std::os::unix::fs::MetadataExt::mode(&metadata) & 0o111 == 0 {
            warn!(
                "'{}' is not executable and therefore not added as an x11 environment",
                file_name
            );

            continue;
        }

        let name = file_name;
        let path = dir_entry.path();
        envs.push(SessionInitializer { name, path });
    }

    envs
}
