use log::warn;

use std::fs;

use super::SessionScript;

const WAYLAND_SESSIONS_DIR: &str = "/etc/lemurs/wayland";

pub fn get_envs() -> Vec<SessionScript> {
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
        envs.push(SessionScript { name, path });
    }

    envs
}
