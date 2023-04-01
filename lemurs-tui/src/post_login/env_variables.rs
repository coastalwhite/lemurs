use log::info;

use env_container::EnvironmentContainer;

use super::PostLoginEnvironment;

pub fn set_display(display: &str, process_env: &mut EnvironmentContainer) {
    info!("Setting Display");

    process_env.set("DISPLAY", display);
}

pub fn set_session_params(
    process_env: &mut EnvironmentContainer,
    post_login_env: &PostLoginEnvironment,
) {
    info!("Setting XDG Session Parameters");

    process_env.set("XDG_SESSION_CLASS", "user");
    process_env.set("XDG_SESSION_TYPE", post_login_env.to_xdg_type());

    // TODO: Implement
    // process_env.set("XDG_CURRENT_DESKTOP", post_login_env.to_xdg_desktop());
    // process_env.set("XDG_SESSION_DESKTOP", post_login_env.to_xdg_desktop());
}

pub fn set_seat_vars(process_env: &mut EnvironmentContainer, tty: u8) {
    info!("Setting XDG Seat Variables");

    process_env.set_or_own("XDG_SEAT", "seat0");
    process_env.set_or_own("XDG_VTNR", &tty.to_string());
}

// NOTE: This uid: u32 might be better set to libc::uid_t
/// Set the XDG environment variables
pub fn set_session_vars(process_env: &mut EnvironmentContainer, uid: u32) {
    info!("Setting XDG Session Variables");

    process_env.set_or_own("XDG_RUNTIME_DIR", &format!("/run/user/{uid}"));
    process_env.set_or_own("XDG_SESSION_ID", "1");
}

/// Set all the environment variables
pub fn set_basic_variables(
    process_env: &mut EnvironmentContainer,
    username: &str,
    homedir: &str,
    shell: &str,
) {
    info!("Setting Basic Environment Variables");

    let pwd = homedir;
    process_env.set_current_dir(pwd);

    process_env.set("HOME", homedir);
    process_env.set("SHELL", shell);
    process_env.set("USER", username);
    process_env.set("LOGNAME", username);
    process_env.set("PATH", "/usr/local/sbin:/usr/local/bin:/usr/bin");

    // process_env.set("MAIL", "..."); TODO: Add
}

pub fn set_xdg_common_paths(process_env: &mut EnvironmentContainer, homedir: &str) {
    info!("Setting XDG Common Paths");

    // This is according to https://wiki.archlinux.org/title/XDG_Base_Directory
    process_env.set("XDG_CONFIG_DIR", &format!("{homedir}/.config"));
    process_env.set("XDG_CACHE_HOME", &format!("{homedir}/.cache"));
    process_env.set("XDG_DATA_HOME", &format!("{homedir}/.local/share"));
    process_env.set("XDG_STATE_HOME", &format!("{homedir}/.local/state"));
    process_env.set("XDG_DATA_DIRS", "/usr/local/share:/usr/share");
    process_env.set("XDG_CONFIG_DIRS", "/etc/xdg");
}
