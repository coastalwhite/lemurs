use log::info;

use pam::{Authenticator, Converse};
use std::env;

use pgs_files::passwd::{get_entry_by_name, PasswdEntry};

pub enum PamError {
    Authentication,
    AccountValidation,
    UsernameNotFound,
    SessionOpen,
}

pub fn init_environment(passwd: &PasswdEntry) {
    info!("Setting environment");

    env::set_var("HOME", &passwd.dir);
    env::set_var("PWD", &passwd.dir);
    env::set_var("SHELL", &passwd.shell);
    env::set_var("USER", &passwd.name);
    env::set_var("LOGNAME", &passwd.name);
    env::set_var("PATH", "/usr/local/sbin:/usr/local/bin:/usr/bin");
    // env::set_var("MAIL", "..."); TODO: Add
}

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

    // Init environment for current TTY
    init_environment(&passwd_entry);

    authenticator
        .open_session()
        .map_err(|_| PamError::SessionOpen)?;

    info!("Opened session");

    // NOTE: Logout happens automatically here with `drop` of session and context
    Ok((authenticator, passwd_entry))
}
