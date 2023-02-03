//! # API Example
//!
//! ```no_run
//! let username = "johndoe";
//! let pasword = "*******";
//!
//! // Define what you want the session to start into
//! let session_environment = SessionEnvironment::X11("/home/johndoe/.xinitrc");
//!
//! // Authenticate the user
//! let session_user = authenticate(
//!     username,
//!     password,
//!     Some(session_environment.session_type())
//! )?;
//!
//! // Start a session environment
//! start_session(session_user, &session_environment);
//! ```
//!
//! # API Expanded
//!
//! ```no_run
//! let username = "johndoe";
//! let pasword = "*******";
//!
//! // Define what you want the session to start into
//! let session_environment = SessionEnvironment::X11("/home/johndoe/.xinitrc");
//!
//! // Authenticate the user
//! let auth_context = AuthContext::default()
//!     .pam_service("system-login");
//! let session_user = authenticate_with_context(
//!     username,
//!     password,
//!     Some(session_environment.session_type())
//!     &auth_context,
//! )?;
//!
//! // Start a session environment
//! let start_env_context = StartEnvContext::default()
//!     .abc();
//! start_session_with_context(session_user, &session_environment, &start_env_context);
//! ```

use env_container::EnvironmentContainer;
use libc::uid_t;
use nix::unistd::{Gid, Uid};
use std::env;
use std::fmt::Display;
use std::process::Child;

use crate::session_environment::env_variables::{
    set_basic_variables, set_display, set_seat_vars, set_session_params, set_session_vars,
    set_xdg_common_paths,
};

use self::auth::{AuthContext, SessionAuthError, SessionUser};
use self::session_environment::{
    EnvironmentStartError, SessionEnvironment, SessionProcess, SessionType,
};

pub mod auth;
pub mod session_environment;

#[derive(Debug, Clone, PartialEq)]
pub struct StartSessionContext {
    pub tty: u8,
}

impl Default for StartSessionContext {
    fn default() -> Self {
        Self { tty: 7 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserInfo {
    username: String,
    user_id: Uid,
    group_id: Gid,
    groups: Vec<Gid>,
    gecos: String,
    home_dir: String,
    shell: String,
}

impl UserInfo {
    /// Get the username of the currently authenticated user
    pub fn username(&self) -> &str {
        &self.username
    }
    /// Get the user id (`uid`) of the currently authenticated user
    pub fn user_id(&self) -> Uid {
        self.user_id
    }
    /// Get the group id (`gid`) of the currently authenticated user
    pub fn group_id(&self) -> Gid {
        self.group_id
    }
    /// Get the group ids (`groups`) of the currently authenticated user
    pub fn groups(&self) -> &[Gid] {
        &self.groups
    }
    /// Get the GECOS User Information of the current authenticated user
    pub fn gecos(&self) -> &str {
        &self.gecos
    }
    /// Get the user home directory of the currently authenticated user
    pub fn home_dir(&self) -> &str {
        &self.home_dir
    }
    /// Get the user shell of the currently authenticated user
    pub fn shell(&self) -> &str {
        &self.shell
    }
}

/// Verify user information are correct and get back a handle to an authenticated user.
///
/// This function asks the operating system to verify the `username` and `password`. If both fields
/// match, the function will fetch further information about the user. The function will then
/// return a [`SessionUser`] handler, which contains all information. If the [`SessionUser`] is
/// dropped, the authenticated session closes.
///
/// In addition to the `username` and `password`, the caller can provide a `session_type`
/// specifying what the caller plans to do with the authenticated session. If it will be used to
/// open a [`SessionEnvironment`], it is recommended to set this field to the corresponding
/// environment. The [`SessionEnvironment::session_type`] method gives the [`SessionType`] for a
/// given [`SessionEnvironment`].
///
/// In contrast to the [`authenticate`] function, this function also allows for finer control of
/// the internal operations with the `context` argument. See the [`AuthContext`] documentation for
/// further explanation of what parameters can be controlled.
pub fn authenticate_with_context<'a>(
    username: &str,
    password: &str,
    session_type: Option<SessionType>,
    context: &AuthContext,
) -> Result<SessionUser<'a>, SessionAuthError> {
    let mut env_container = EnvironmentContainer::take_snapshot();

    set_display(&mut env_container);
    set_session_params(&mut env_container, session_type);

    SessionUser::authenticate_with_context(username, password, env_container, &context)
}

/// Verify user information are correct and get back a handle to an authenticated user.
///
/// This function asks the operating system to verify the `username` and `password`. If both fields
/// match, the function will fetch further information about the user. The function will then
/// return a [`SessionUser`] handler, which contains all information. If the [`SessionUser`] is
/// dropped, the authenticated session closes.
///
/// In addition to the `username` and `password`, the caller can provide a `session_type`
/// specifying what the caller plans to do with the authenticated session. If it will be used to
/// open a [`SessionEnvironment`], it is recommended to set this field to the corresponding
/// environment. The [`SessionEnvironment::session_type`] method gives the [`SessionType`] for a
/// given [`SessionEnvironment`].
///
/// This function does not provide fine-grain control over the internal parameters and instead uses
/// the default [`AuthContext`]. The default settings can be found in the documentation of
/// [`AuthContext`]. If more control is needed, the [`authenticate_with_context`] can be used.
pub fn authenticate<'a>(
    username: &str,
    password: &str,
    session_type: Option<SessionType>,
) -> Result<SessionUser<'a>, SessionAuthError> {
    authenticate_with_context(username, password, session_type, &AuthContext::default())
}

/// Open a `session_environment` with the given `user_info` and return a handler to the opened
/// process.
///
/// **NOTE**: If you have previously called [`authenticate`], [`authenticate_with_context`] or
/// created a [`SessionUser`] in another way, you should use the
/// [`open_authenticated_session_with_context`] function instead of this one.
///
/// This function will generate the proper environment and processes to open the given
/// `session_environment` with the user information in `user_info`. It will return a handler to all
/// the created processes that can be operated upon. If you want to go into the session immediately
/// afterwards, it is recommended to call [`SessionProcess::wait`] on the result.
///
/// In contrast to the [`open_session`] function, this function also allows for finer control of
/// the internal operations with the `context` argument. See the [`StartSessionContext`]
/// documentation for further explanation of what parameters can be controlled.
pub fn open_session_with_context(
    user_info: &UserInfo,
    session_environment: &SessionEnvironment,
    context: &StartSessionContext,
) -> Result<SessionProcess<Child>, EnvironmentStartError> {
    let tty = context.tty;

    let mut env_container = EnvironmentContainer::take_snapshot();

    let username = user_info.username();
    let uid = user_info.user_id();
    let homedir = &user_info.home_dir();
    let shell = &user_info.shell();

    set_seat_vars(&mut env_container, tty);
    set_session_vars(&mut env_container, uid);
    set_basic_variables(&mut env_container, username, homedir, shell);
    set_xdg_common_paths(&mut env_container, homedir);

    session_environment.spawn(user_info)
}

/// Open a `session_environment` with the given `user_info` and return a handler to the opened
/// process.
///
/// **NOTE**: If you have previously called [`authenticate`], [`authenticate_with_context`] or
/// created a [`SessionUser`] in another way, you should use the [`open_authenticated_session`]
/// function instead of this one.
///
/// This function will generate the proper environment and processes to open the given
/// `session_environment` with the user information in `user_info`. It will return a handler to all
/// the created processes that can be operated upon. If you want to go into the session immediately
/// afterwards, it is recommended to call [`SessionProcess::wait`] on the result.
///
/// This function does not provide fine-grain control over the internal parameters and instead uses
/// the default [`StartSessionContext`]. The default settings can be found in the documentation of
/// [`StartSessionContext`]. If more control is needed, the [`open_session_with_context`] can be
/// used.
pub fn open_session(
    user_info: &UserInfo,
    session_environment: &SessionEnvironment,
) -> Result<SessionProcess<Child>, EnvironmentStartError> {
    let context = StartSessionContext::default();
    open_session_with_context(user_info, session_environment, &context)
}

/// Open a `session_environment` fore the given `session_user` and return a handler to the opened
/// process.
///
/// This function will generate the proper environment and processes to open the given
/// `session_environment` with the `session_user`. It will return a handler to all the created
/// processes that can be operated upon. If you want to go into the session immediately afterwards,
/// it is recommended to call [`SessionProcess::wait`] on the result.
///
/// In contrast to the [`open_authenticated_session`] function, this function also allows for finer
/// control of the internal operations with the `context` argument. See the [`StartSessionContext`]
/// documentation for further explanation of what parameters can be controlled.
pub fn open_authenticated_session_with_context<'a>(
    session_user: SessionUser<'a>,
    session_environment: &SessionEnvironment,
    context: &StartSessionContext,
) -> Result<SessionProcess<Child>, EnvironmentStartError> {
    let session_process =
        open_session_with_context(session_user.as_user_info(), session_environment, context)?;

    // Insert the pid into the UTMPX entry, if needed
    session_user.set_pid(session_process.pid());

    Ok(session_process)
}

/// Open a `session_environment` fore the given `session_user` and return a handler to the opened
/// process.
///
/// This function will generate the proper environment and processes to open the given
/// `session_environment` with the `session_user`. It will return a handler to all the created
/// processes that can be operated upon. If you want to go into the session immediately afterwards,
/// it is recommended to call [`SessionProcess::wait`] on the result.
///
/// This function does not provide fine-grain control over the internal parameters and instead uses
/// the default [`StartSessionContext`]. The default settings can be found in the documentation of
/// [`StartSessionContext`]. If more control is needed, the
/// [`open_authenticated_session_with_context`] can be used.
pub fn open_authenticated_session<'a>(
    session_user: SessionUser<'a>,
    session_environment: &SessionEnvironment,
) -> Result<SessionProcess<Child>, EnvironmentStartError> {
    let context = StartSessionContext::default();
    open_authenticated_session_with_context(session_user, session_environment, &context)
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
