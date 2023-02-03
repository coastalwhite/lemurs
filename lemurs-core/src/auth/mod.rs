mod pam;
mod utmpx;

use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;

use env_container::EnvironmentContainer;
use libc::{c_char, pid_t, utmpx as Utmpx};
use log::info;
use nix::unistd::{Gid, Uid};

use pgs_files::passwd::get_entry_by_name;
use users::get_user_groups;

use crate::{can_run, RunError, UserInfo};

use self::pam::{PamError, PamSession};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    UsernameNotFound,
}

#[derive(Debug, Clone)]
pub enum SessionAuthError {
    Run(RunError),
    Authentication(AuthError),
    Pam(PamError),
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    backend_specific: <AuthBackend<'static> as AuthSession>::Context,
    session_tty: u8,
    use_utmpx: bool,
}

/// Integrated backends. These all allow to open a session given a username and password
/// credential.
type AuthBackend<'a> = PamSession<'a>;

trait AuthSession: Sized {
    type Err: Error + Sized;
    type Context: Default + Sized;

    fn open(
        username: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<Self, SessionAuthError> {
        Self::open_with_context(username, password, &Self::Context::default())
    }

    fn open_with_context(
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        context: &Self::Context,
    ) -> Result<Self, SessionAuthError>;
}

/// The information of a user currently within a session. If this structure is dropped then the
/// session is also ended.
pub struct SessionUser<'a> {
    // This is used to keep the user session. If the struct is dropped then the user session is
    // also automatically dropped.
    #[allow(dead_code)]
    session: AuthBackend<'a>,

    #[allow(dead_code)]
    env_container: EnvironmentContainer,

    utmpx: Option<RefCell<Utmpx>>,

    info: UserInfo,
}

impl<'a> SessionUser<'a> {
    pub fn as_user_info(&self) -> &UserInfo {
        &self.info
    }

    /// Attempt to create a new authenticated user from their username and password.
    pub fn authenticate(username: &'_ str, password: &'_ str, env_container: EnvironmentContainer) -> Result<Self, SessionAuthError> {
        let auth_context = AuthContext::default();
        Self::authenticate_with_context(username, password, env_container, &auth_context)
    }

    /// Attempt to create a new authenticated user from their username and password with an
    /// arbitrary authentication context.
    pub fn authenticate_with_context(
        username: &'_ str,
        password: &'_ str,
        env_container: EnvironmentContainer,
        auth_context: &AuthContext,
    ) -> Result<Self, SessionAuthError> {
        can_run()?;

        let session =
            AuthBackend::open_with_context(username, password, &auth_context.backend_specific)?;
        let session = session.into();

        // NOTE: Maybe we should also load all groups here
        let info = get_entry_by_name(&username).ok_or(AuthError::UsernameNotFound)?;

        let groups: Vec<Gid> = get_user_groups(&info.name, info.gid)
            .unwrap()
            .iter()
            .map(|group| Gid::from_raw(group.gid()))
            .collect();

        let utmpx = auth_context
            .use_utmpx
            .then_some(RefCell::new(utmpx::add_utmpx_entry(
                username,
                auth_context.session_tty,
            )));

        let info = UserInfo {
            username: String::from(username),
            user_id: Uid::from_raw(info.uid),
            group_id: Gid::from_raw(info.gid),
            groups,
            gecos: String::from(info.gecos),
            home_dir: String::from(info.dir),
            shell: String::from(info.shell),
        };

        Ok(Self {
            session,
            env_container,
            utmpx,
            info,
        })
    }

    pub fn set_pid(&self, pid: u32) {
        if let Some(utmpx) = &self.utmpx {
            utmpx.replace_with(|utmpx| {
                utmpx.ut_pid = pid as pid_t;

                unsafe {
                    libc::setutxent();
                    libc::pututxline(utmpx as *const Utmpx);
                };

                *utmpx
            });
        }
    }
}

impl<'a> Drop for SessionUser<'a> {
    fn drop(&mut self) {
        if let Some(utmpx) = &self.utmpx {
            info!("Removing UTMPX record");

            utmpx.replace_with(|utmpx| {
                utmpx.ut_type = libc::DEAD_PROCESS;

                utmpx.ut_line = <[c_char; 32]>::default();
                utmpx.ut_user = <[c_char; 32]>::default();

                utmpx.ut_tv.tv_usec = 0;
                utmpx.ut_tv.tv_sec = 0;

                unsafe {
                    libc::setutxent();
                    libc::pututxline(utmpx as *const Utmpx);
                    libc::endutxent();
                }

                *utmpx
            });
        }
    }
}


impl Default for AuthContext {
    fn default() -> Self {
        Self {
            backend_specific: <AuthBackend<'static> as AuthSession>::Context::default(),
            session_tty: 1,
            use_utmpx: true,
        }
    }
}

impl Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use AuthError::*;

        match self {
            InvalidCredentials => write!(f, "Invalid login credentials"),
            UsernameNotFound => {
                write!(f, "Login creditionals are valid, but username is not found")
            }
        }
    }
}

impl From<RunError> for SessionAuthError {
    fn from(value: RunError) -> Self {
        SessionAuthError::Run(value)
    }
}

impl From<AuthError> for SessionAuthError {
    fn from(value: AuthError) -> Self {
        SessionAuthError::Authentication(value)
    }
}

impl From<PamError> for SessionAuthError {
    fn from(value: PamError) -> Self {
        SessionAuthError::Pam(value)
    }
}
