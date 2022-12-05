use log::{error, info};
use std::env;

use super::PostLoginEnvironment;

fn env_set_and_announce(key: &str, value: &str) {
    env::set_var(key, value);
    info!("Set environment variable '{}' to '{}'", key, value);
}

/// Set all the environment variables
pub fn init_environment(username: &str, homedir: &str, shell: &str) {
    env_set_and_announce("HOME", homedir);

    let pwd = homedir;
    if env::set_current_dir(pwd).is_ok() {
        info!("Successfully changed working directory to {}!", pwd);
    } else {
        error!("Failed to change the working directory to {}", pwd);
    }

    env_set_and_announce("SHELL", shell);
    env_set_and_announce("USER", username);
    env_set_and_announce("LOGNAME", username);
    env_set_and_announce("PATH", "/usr/local/sbin:/usr/local/bin:/usr/bin");

    // env::set_var("MAIL", "..."); TODO: Add
}

// NOTE: This uid: u32 might be better set to libc::uid_t
/// Set the XDG environment variables
pub fn set_xdg_env(uid: u32, homedir: &str, tty: u8, post_login_env: &PostLoginEnvironment) {
    // This is according to https://wiki.archlinux.org/title/XDG_Base_Directory

    env_set_and_announce("XDG_CONFIG_DIR", &format!("{}/.config", homedir));
    env_set_and_announce("XDG_CACHE_HOME", &format!("{}/.cache", homedir));
    env_set_and_announce("XDG_DATA_HOME", &format!("{}/.local/share", homedir));
    env_set_and_announce("XDG_STATE_HOME", &format!("{}/.local/state", homedir));
    env_set_and_announce("XDG_DATA_DIRS", "/usr/local/share:/usr/share");
    env_set_and_announce("XDG_CONFIG_DIRS", "/etc/xdg");

    env_set_and_announce("XDG_RUNTIME_DIR", &format!("/run/user/{}", uid));
    env_set_and_announce("XDG_SESSION_DIR", "user");
    env_set_and_announce("XDG_SESSION_ID", "1");
    env_set_and_announce("XDG_SEAT", "seat0");
    env_set_and_announce("XDG_VTNR", &tty.to_string());

    env_set_and_announce(
        "XDG_SESSION_TYPE",
        match post_login_env {
            PostLoginEnvironment::Shell => "tty",
            PostLoginEnvironment::X { .. } => "x11",
            PostLoginEnvironment::Wayland { .. } => "wayland",
        },
    );
}
