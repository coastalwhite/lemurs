mod pam;

use ::pam::{Authenticator, PasswordConv};
use log::info;

use crate::auth::pam::open_session;
pub use crate::auth::pam::AuthenticationError;

pub struct AuthUserInfo<'a> {
    // This is used to keep the user session. If the struct is dropped then the user session is
    // also automatically dropped.
    #[allow(dead_code)]
    authenticator: Authenticator<'a, PasswordConv>,

    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub gecos: String,
    pub dir: String,
    pub shell: String,
}

pub fn try_auth<'a>(
    username: &str,
    password: &str,
    pam_service: &str,
) -> Result<AuthUserInfo<'a>, AuthenticationError> {
    info!("Login attempt for '{username}'");

    open_session(username, password, pam_service)
        .map(|(authenticator, entry)| AuthUserInfo {
            authenticator,
            name: entry.name,
            uid: entry.uid,
            gid: entry.gid,
            gecos: entry.gecos,
            dir: entry.dir,
            shell: entry.shell,
        })
        .map_err(|err| {
            info!(
                "Authentication failed for '{}'. Reason: {}",
                username,
                err.to_string()
            );
            err
        })
}
