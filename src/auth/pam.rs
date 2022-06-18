use log::info;

use pam::{Authenticator, PasswordConv};

const PAM_SERVICE: &str = "login";

use pgs_files::passwd::{get_entry_by_name, PasswdEntry};

/// All the different errors that can occur during PAM opening an authenticated session
#[derive(Clone, Copy)]
pub enum AuthenticationError {
    PamService,
    AccountValidation,
    UsernameNotFound,
    SessionOpen,
}

impl ToString for AuthenticationError {
    fn to_string(&self) -> String {
        match self {
            AuthenticationError::PamService => format!("Failed to create authenticator with PAM service '{}'", PAM_SERVICE),
            AuthenticationError::AccountValidation => "Invalid login credentials".to_string(),
            AuthenticationError::UsernameNotFound => "Login creditionals are valid, but username is not found. This should not be possible :(".to_string(),
            AuthenticationError::SessionOpen => "Failed to open a PAM session".to_string(),
        }
    }
}

/// Open a PAM authenticated session
pub fn open_session<'a>(
    username: impl ToString,
    password: impl ToString,
) -> Result<(Authenticator<'a, PasswordConv>, PasswdEntry), AuthenticationError> {
    let username = username.to_string();
    let password = password.to_string();

    info!("Started opening session");

    let mut authenticator = Authenticator::with_password(PAM_SERVICE)
        .map_err(|_| AuthenticationError::PamService)?;

    info!("Gotten Authenticator");

    // Authenticate the user
    authenticator
        .get_handler()
        .set_credentials(&username, &password);

    info!("Got handler");

    // Validate the account
    authenticator
        .authenticate()
        .map_err(|_| AuthenticationError::AccountValidation)?;

    info!("Validated account");

    // NOTE: Maybe we should also load all groups here
    let passwd_entry = get_entry_by_name(&username).ok_or(AuthenticationError::UsernameNotFound)?;

    authenticator
        .open_session()
        .map_err(|_| AuthenticationError::SessionOpen)?;

    info!("Opened session");

    // NOTE: Logout happens automatically here with `drop` of session and context
    Ok((authenticator, passwd_entry))
}
