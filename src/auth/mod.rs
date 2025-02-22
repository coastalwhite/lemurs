mod pam;
pub mod utmpx;

use ::pam::{Authenticator, PasswordConv};
use log::info;

use crate::auth::pam::open_session;
pub use crate::auth::pam::AuthenticationError;

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
    pub home_dir: String,
    pub shell: String,
}

pub fn try_auth<'a>(
    username: &str,
    password: &str,
    pam_service: &str,
) -> Result<AuthUserInfo<'a>, AuthenticationError> {
    info!("Login attempt for '{username}'");

    open_session(username, password, pam_service).inspect_err(|err| {
        info!(
            "Authentication failed for '{}'. Reason: {}",
            username,
            err.to_string()
        );
    })
}
