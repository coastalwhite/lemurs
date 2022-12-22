use log::{info, warn};

use std::fs;
use std::process::Command;

use crate::auth::SessionUser;

use super::{EnvironmentContext, SessionInitializer};

const WAYLAND_SESSIONS_DIR: &str = "/etc/lemurs/wayland";

pub enum WaylandStartError {
    InitializerStartFailed,
}

pub struct WaylandStartContext<'a> {
    system_shell: &'a str,
}

impl Default for WaylandStartContext<'static> {
    fn default() -> Self {
        (&EnvironmentContext::default()).into()
    }
}

impl<'a> From<&EnvironmentContext<'a>> for WaylandStartContext<'a> {
    fn from(context: &EnvironmentContext<'a>) -> Self {
        let EnvironmentContext { system_shell, .. } = context;
        Self { system_shell }
    }
}

impl SessionInitializer {
    pub fn start_wayland(
        &self,
        _session_user: &SessionUser<'_>,
        context: &WaylandStartContext,
    ) -> Result<Command, WaylandStartError> {
        info!("Starting Wayland session '{}'", self.name);

        let mut initializer = Command::new(context.system_shell);

        // Make it run the initializer
        initializer.arg("-c").arg(&self.path);

        Ok(initializer)
    }
}

pub fn get_envs() -> Vec<SessionInitializer> {
    let Ok(dir_entries) = fs::read_dir(WAYLAND_SESSIONS_DIR) else {
        warn!(
            "Failed to read from the wayland sessions folder '{}'",
            WAYLAND_SESSIONS_DIR
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
            warn!("Ignored errorinous wayland path: '{}'", dir_entry.unwrap_err());
            continue;
        };

        // Check UTF-8 compatability of file_name
        let Ok(file_name) = dir_entry.file_name().into_string() else {
            warn!("Unable to convert OSString to String. Skipping wayland item");
            continue;
        };

        // Get file metadata
        let Ok(metadata) = dir_entry.metadata() else {
            warn!("Unable to convert OSString to String. Skipping wayland item");
            continue;
        };

        // Make sure the file is executable
        if std::os::unix::fs::MetadataExt::mode(&metadata) & 0o111 == 0 {
            warn!(
                "'{}' is not executable and therefore not added as an wayland environment",
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
