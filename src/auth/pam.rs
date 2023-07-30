use log::info;

use pam::Authenticator;
use users::os::unix::UserExt;

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

impl ToString for AuthenticationError {
    fn to_string(&self) -> String {
        match self {
            AuthenticationError::PamService(service) => format!("Failed to create authenticator with PAM service '{service}'"),
            AuthenticationError::AccountValidation => "Invalid login credentials".to_string(),
            AuthenticationError::HomeDirInvalidUtf8 => "User home directory path contains invalid UTF-8".to_string(),
            AuthenticationError::ShellInvalidUtf8 => "User shell path contains invalid UTF-8".to_string(),
            AuthenticationError::UsernameNotFound => "Login creditionals are valid, but username is not found. This should not be possible :(".to_string(),
            AuthenticationError::SessionOpen => "Failed to open a PAM session".to_string(),
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

    let user = users::get_user_by_name(username).ok_or(AuthenticationError::UsernameNotFound)?;

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
