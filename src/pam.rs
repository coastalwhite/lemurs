use log::{error, info};
use std::path::PathBuf;

use pam::{Authenticator, Converse};
use std::env;

use pgs_files::group::get_all_entries;
use pgs_files::passwd::{get_entry_by_name, PasswdEntry};

const SYSTEM_SHELL: &str = "/bin/sh";

pub enum PamError {
    FailedToSpawn, // TODO: Add io::Result
    Authentication,
    AccountValidation,
    UsernameNotFound,
    SessionOpen,
    EnvironmentError,
    Child,
}

pub fn init_environment(passwd: &PasswdEntry) {
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
) -> Result<(Authenticator<'a, impl Converse>, PasswdEntry, Vec<u32>), PamError> {
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

    info!("Validated");

    // NOTE: Maybe we should also load all groups here
    let passwd_entry = get_entry_by_name(&username).ok_or(PamError::UsernameNotFound)?;
    let groups = get_all_entries()
        .into_iter()
        .filter(|entry| entry.members.contains(&username))
        .map(|entry| entry.gid)
        .collect();
    // Init environment for current TTY
    init_environment(&passwd_entry);

    info!("Initiated environment");

    authenticator
        .open_session()
        .map_err(|_| PamError::SessionOpen)?;

    info!("Opened session");

    // NOTE: Logout happens automatically here with `drop` of session and context
    Ok((authenticator, passwd_entry, groups))
}
