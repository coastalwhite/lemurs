use libc::uid_t;
use log::error;
use std::env;
use std::fmt::Display;

pub mod session_environment;
pub mod auth;

macro_rules! log_error_and_return {
    ($error_variant:expr) => {
        let error = $error_variant;
        error!("{}", error);
        return Err(error);
    };
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RunError {
    NonRootUser(uid_t),
    AlreadyInSession,
}

const ROOT_UID: uid_t = 0;

impl Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NonRootUser(uid) => write!(
                f,
                "Not ran as root. Found user id '{}' instead of '{}'",
                uid, ROOT_UID
            ),
            RunError::AlreadyInSession => write!(
                f,
                "Lemurs ran again in an existing session. Namely, `XDG_SESSION_TYPE` is set."
            ),
        }
    }
}

pub fn can_run() -> Result<(), RunError> {
    let uid = users::get_current_uid();
    if uid != ROOT_UID {
        log_error_and_return!(RunError::NonRootUser(uid));
    }

    if env::var("XDG_SESSION_TYPE").is_ok() {
        log_error_and_return!(RunError::AlreadyInSession);
    }

    return Ok(());
}
