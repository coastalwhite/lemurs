use env_container::EnvironmentContainer;
use libc::uid_t;
use log::info;
use std::env;
use std::fmt::Display;

use crate::session_environment::env_variables::{
    set_basic_variables, set_display, set_seat_vars, set_session_params, set_session_vars,
    set_xdg_common_paths,
};

use self::auth::{SessionAuthError, SessionUser};
use self::session_environment::{EnvironmentStartError, SessionEnvironment, SessionType};

pub mod auth;
pub mod session_environment;

struct Config {
    tty: u8,
}

pub fn authenticate<'a>(
    username: &'_ str,
    password: &'_ str,
    session_type: Option<SessionType>,
) -> Result<SessionUser<'a>, SessionAuthError> {
    let mut env_container = EnvironmentContainer::take_snapshot();

    set_display(&mut env_container);
    set_session_params(&mut env_container, session_type);

    SessionUser::authenticate(username, password, env_container)
}

fn start_session<'a>(
    mut session_user: SessionUser<'a>,
    session_environment: &SessionEnvironment,
    config: &Config,
) -> Result<(), EnvironmentStartError> {
    let tty = config.tty;

    let mut env_container = session_user
        .take_env_container()
        .ok_or(EnvironmentStartError::ReusedSessionUser)?;

    let username = session_user.username();
    let uid = session_user.user_id();
    let homedir = &session_user.home_dir();
    let shell = &session_user.shell();

    set_seat_vars(&mut env_container, tty);
    set_session_vars(&mut env_container, uid);
    set_basic_variables(&mut env_container, username, homedir, shell);
    set_xdg_common_paths(&mut env_container, homedir);

    let spawned_environment = session_environment.spawn(&mut session_user)?;

    drop(env_container);

    info!("Waiting for environment to terminate");

    spawned_environment.wait()?;

    info!("Environment terminated. Returning to Lemurs...");

    drop(session_user);

    Ok(())
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RunError {
    NonRootUser(uid_t),
    AlreadyInSession,
}

const ROOT_UID: uid_t = 0;

impl Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RunError::*;

        match self {
            NonRootUser(uid) => write!(
                f,
                "Not ran as root. Found user id '{}' instead of '{}'",
                uid, ROOT_UID
            ),
            AlreadyInSession => write!(
                f,
                "Ran in an existing session. Namely, `XDG_SESSION_TYPE` is set."
            ),
        }
    }
}

/// Verify whether lemurs can be running in the current environment or not. This function is ran at
/// the beginning of most of the public API functions. It verifies two conditions:
/// 1. Is the current user the root user?
/// 2. Are we in an existing session?
pub fn can_run() -> Result<(), RunError> {
    let uid = users::get_current_uid();
    if uid != ROOT_UID {
        return Err(RunError::NonRootUser(uid));
    }

    if env::var("XDG_SESSION_TYPE").is_ok() {
        return Err(RunError::AlreadyInSession);
    }

    Ok(())
}
