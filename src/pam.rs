use log::{error, info};

use pam::{Authenticator, Converse};
use std::env;
use std::path::Path;

use pgs_files::passwd::{get_entry_by_name, PasswdEntry};

/// All the different errors that can occur during PAM opening an authenticated session
pub enum PamError {
    Authentication,
    AccountValidation,
    UsernameNotFound,
    SessionOpen,
}

/// Open a PAM authenticated session
pub fn open_session<'a>(
    username: impl ToString,
    password: impl ToString,
) -> Result<(Authenticator<'a, impl Converse>, PasswdEntry), PamError> {
    let username = username.to_string();
    let password = password.to_string();

    info!("Started opening session");

    let mut authenticator = Authenticator::with_password(
        "login", // Service name
    )
    .map_err(|_| PamError::Authentication)?;

    info!("Gotten Authenticator");

    // Authenticate the user
    authenticator
        .get_handler()
        .set_credentials(&username, &password);

    info!("Got handler");

    // Validate the account
    authenticator
        .authenticate()
        .map_err(|_| PamError::AccountValidation)?;

    info!("Validated account");

    // NOTE: Maybe we should also load all groups here
    let passwd_entry = get_entry_by_name(&username).ok_or(PamError::UsernameNotFound)?;

    authenticator
        .open_session()
        .map_err(|_| PamError::SessionOpen)?;

    info!("Opened session");

    // NOTE: Logout happens automatically here with `drop` of session and context
    Ok((authenticator, passwd_entry))
}
