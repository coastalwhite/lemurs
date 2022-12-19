use libc::uid_t;
use std::env;
use std::fmt::Display;

pub mod auth;
pub mod session_environment;

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
