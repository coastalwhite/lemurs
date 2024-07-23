use std::fmt;

use log::info;

use pam::Authenticator;
use uzers::os::unix::UserExt;

use crate::auth::AuthUserInfo;

/// All the different errors that can occur during PAM opening an authenticated session
#[derive(Clone)]
pub enum AuthenticationError {
    PamService(String),
    AccountValidation,
    HomeDirInvalidUtf8,
    ShellInvalidUtf8,
    UsernameNotFound,
    SessionOpen,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PamService(service) => write!(f, "Failed to create authenticator with PAM service '{service}'"),
            Self::AccountValidation => f.write_str("Invalid login credentials"),
            Self::HomeDirInvalidUtf8 => f.write_str("User home directory path contains invalid UTF-8"),
            Self::ShellInvalidUtf8 => f.write_str("User shell path contains invalid UTF-8"),
            Self::UsernameNotFound => f.write_str("Login creditionals are valid, but username is not found. This should not be possible :("),
            Self::SessionOpen => f.write_str("Failed to open a PAM session"),
        }
    }
}

/// Open a PAM authenticated session
pub fn open_session<'a>(
    username: &str,
    password: &str,
    pam_service: &str,
) -> Result<AuthUserInfo<'a>, AuthenticationError> {
    info!("Started opening session");

    let mut authenticator = Authenticator::with_password(pam_service)
        .map_err(|_| AuthenticationError::PamService(pam_service.to_string()))?;

    info!("Gotten Authenticator");

    // Authenticate the user
    authenticator
        .get_handler()
        .set_credentials(username, password);

    info!("Got handler");

    // Validate the account
    authenticator
        .authenticate()
        .map_err(|_| AuthenticationError::AccountValidation)?;

    info!("Validated account");

    let user = uzers::get_user_by_name(username).ok_or(AuthenticationError::UsernameNotFound)?;

    let uid = user.uid();
    let primary_gid = user.primary_group_id();
    let all_gids = user.groups().map_or_else(Vec::default, |v| {
        v.into_iter().map(|group| group.gid()).collect()
    });
    let home_dir = user
        .home_dir()
        .to_str()
        .ok_or(AuthenticationError::HomeDirInvalidUtf8)?
        .to_string();
    let shell = user
        .shell()
        .to_str()
        .ok_or(AuthenticationError::ShellInvalidUtf8)?
        .to_string();

    authenticator
        .open_session()
        .map_err(|_| AuthenticationError::SessionOpen)?;

    info!("Opened session");

    // NOTE: Logout happens automatically here with `drop` of authenticator
    Ok(AuthUserInfo {
        authenticator,

        username: username.to_string(),
        uid,
        primary_gid,
        all_gids,
        home_dir,
        shell,
    })
}
