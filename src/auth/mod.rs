mod pam;
pub mod utmpx;

use ::pam::{Authenticator, PasswordConv};
use log::info;

use crate::auth::pam::validate_credentials;
pub use crate::auth::pam::AuthenticationError;
pub use crate::auth::pam::open_session;

/// Holds an authenticated (but not yet session-opened) PAM handle plus user account info.
///
/// Pass this across a `fork()` boundary: the child calls `open_session` on it, then execs the
/// compositor.  The parent calls `std::mem::forget` so the PAM handle is never double-freed.
pub struct ValidatedCredentials<'a> {
    pub(crate) authenticator: Authenticator<'a, PasswordConv>,

    pub username: String,
    pub uid: libc::uid_t,
    pub primary_gid: libc::gid_t,
    pub all_gids: Vec<libc::gid_t>,
    pub home_dir: String,
    pub shell: String,
}

pub struct AuthUserInfo<'a> {
    // This is used to keep the user session. If the struct is dropped then the user session is
    // also automatically dropped.
    #[allow(dead_code)]
    authenticator: Authenticator<'a, PasswordConv>,

    #[allow(dead_code)]
    pub username: String,

    pub uid: libc::uid_t,
    pub primary_gid: libc::gid_t,
    pub all_gids: Vec<libc::gid_t>,
    pub shell: String,
}

pub fn try_validate<'a>(
    username: &str,
    password: &str,
    pam_service: &str,
) -> Result<ValidatedCredentials<'a>, AuthenticationError> {
    info!("Login attempt for '{username}'");

    validate_credentials(username, password, pam_service).inspect_err(|err| {
        info!(
            "Authentication failed for '{}'. Reason: {}",
            username,
            err.to_string()
        );
    })
}
