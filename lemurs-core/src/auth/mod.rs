mod pam;
pub mod utmpx;

use std::cell::RefCell;

use ::pam::{Authenticator, PasswordConv};
use libc::{c_char, pid_t, utmpx as Utmpx};
use log::info;
use nix::unistd::{Gid, Uid};

use crate::auth::pam::open_session;
pub use crate::auth::pam::AuthenticationError;

pub struct AuthContext<'a> {
    pam_service: &'a str,
}

impl Default for AuthContext<'static> {
    fn default() -> Self {
        Self {
            pam_service: "system-login",
        }
    }
}

/// The information of a user currently within a session. If this structure is dropped then the
/// session is also ended.
pub struct SessionUser<'a> {
    // This is used to keep the user session. If the struct is dropped then the user session is
    // also automatically dropped.
    #[allow(dead_code)]
    authenticator: Authenticator<'a, PasswordConv>,

    username: String,
    user_id: Uid,
    group_id: Gid,
    groups: Vec<Gid>,
    gecos: String,
    home_dir: String,
    shell: String,
    utmpx: RefCell<Utmpx>,
}

impl<'a> SessionUser<'a> {
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

    /// Attempt to create a new authenticated user from their username and password.
    pub fn authenticate(username: &'_ str, password: &'_ str) -> Result<Self, AuthenticationError> {
        let auth_context = AuthContext::default();
        Self::authenticate_with_context(username, password, &auth_context)
    }

    /// Attempt to create a new authenticated user from their username and password with an
    /// arbitrary authentication context.
    pub fn authenticate_with_context(
        username: &'_ str,
        password: &'_ str,
        auth_context: &AuthContext,
    ) -> Result<Self, AuthenticationError> {
        unimplemented!()
    }

    pub fn set_pid(&self, pid: u32) {
        self.utmpx.replace_with(|utmpx| {
            utmpx.ut_pid = pid as pid_t;

            unsafe {
                libc::setutxent();
                libc::pututxline(utmpx as *const Utmpx);
            };

            *utmpx
        });
    }
}

impl<'a> Drop for SessionUser<'a> {
    fn drop(&mut self) {
        info!("Removing UTMPX record");

        self.utmpx.replace_with(|utmpx| {
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

pub fn try_auth<'a>(
    username: String,
    password: String,
) -> Result<SessionUser<'a>, AuthenticationError> {
    info!("Login attempt for '{}'", username);

    unimplemented!()

    // open_session(username.clone(), password)
    //     .map(|(authenticator, entry)| SessionUser {
    //         authenticator,
    //         username: entry.name,
    //         user_id: entry.uid,
    //         group_id: entry.gid,
    //         groups: Vec::new(), // TODO: Add the groups
    //         gecos: entry.gecos,
    //         home_dir: entry.dir,
    //         shell: entry.shell,
    //     })
    //     .map_err(|err| {
    //         info!(
    //             "Authentication failed for '{}'. Reason: {}",
    //             username,
    //             err.to_string()
    //         );
    //         err
    //     })
}
