use log::{error, info};

use super::PostLoginEnvironment;

pub fn set_display(display: &str) {
    info!("Setting Display");

    std::env::set_var("DISPLAY", display);
}

pub fn remove_xdg() {
    info!("Clearing XDG preemptively to set later");

    std::env::remove_var("XDG_SESSION_CLASS");
    std::env::remove_var("XDG_CURRENT_DESKTOP");
    std::env::remove_var("XDG_SESSION_DESKTOP");

    std::env::remove_var("XDG_SEAT");
    std::env::remove_var("XDG_VTNR");

    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("XDG_SESSION_ID");

    std::env::remove_var("XDG_CONFIG_DIR");
    std::env::remove_var("XDG_CACHE_HOME");
    std::env::remove_var("XDG_DATA_HOME");
    std::env::remove_var("XDG_STATE_HOME");
    std::env::remove_var("XDG_DATA_DIRS");
    std::env::remove_var("XDG_CONFIG_DIRS");
}

pub fn set_session_params(post_login_env: &PostLoginEnvironment) {
    info!("Setting XDG Session Parameters");

    std::env::set_var("XDG_SESSION_CLASS", "user");
    std::env::set_var("XDG_SESSION_TYPE", post_login_env.to_xdg_type());

    // TODO: Implement
    // process_env.set("XDG_CURRENT_DESKTOP", post_login_env.to_xdg_desktop());
    // process_env.set("XDG_SESSION_DESKTOP", post_login_env.to_xdg_desktop());
}

pub fn set_or_own_env(key: &'static str, value: &str) {
    if std::env::var(key) == Err(std::env::VarError::NotPresent) {
        std::env::set_var(key, value);
    }
}

pub fn set_seat_vars(tty: u8) {
    info!("Setting XDG Seat Variables");

    set_or_own_env("XDG_SEAT", "seat0");
    set_or_own_env("XDG_VTNR", &tty.to_string());
}

// NOTE: This uid: u32 might be better set to libc::uid_t
/// Set the XDG environment variables
pub fn set_session_vars(uid: u32) {
    info!("Setting XDG Session Variables");

    set_or_own_env("XDG_RUNTIME_DIR", &format!("/run/user/{uid}"));
    set_or_own_env("XDG_SESSION_ID", "1");
}

/// Set all the environment variables
pub fn set_basic_variables(username: &str, homedir: &str, shell: &str, path: &str) {
    info!("Setting Basic Environment Variables");

    let pwd = homedir;
    if std::env::set_current_dir(pwd).is_err() {
        error!("Failed to set current working directory.");
    }

    std::env::set_var("HOME", homedir);
    std::env::set_var("SHELL", shell);
    std::env::set_var("USER", username);
    std::env::set_var("LOGNAME", username);
    std::env::set_var("PATH", path);

    // process_env.set("MAIL", "..."); TODO: Add
}

pub fn set_xdg_common_paths(homedir: &str) {
    info!("Setting XDG Common Paths");

    // This is according to https://wiki.archlinux.org/title/XDG_Base_Directory
    set_or_own_env("XDG_CONFIG_HOME", &format!("{homedir}/.config"));
    set_or_own_env("XDG_CACHE_HOME", &format!("{homedir}/.cache"));
    set_or_own_env("XDG_DATA_HOME", &format!("{homedir}/.local/share"));
    set_or_own_env("XDG_STATE_HOME", &format!("{homedir}/.local/state"));
    set_or_own_env("XDG_DATA_DIRS", "/usr/local/share:/usr/share");
    set_or_own_env("XDG_CONFIG_DIRS", "/etc/xdg");
}
