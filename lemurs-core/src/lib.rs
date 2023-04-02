pub mod auth;
pub mod post_login;

use auth::{try_auth, AuthenticationError};
use post_login::env_variables::{
    set_basic_variables, set_display, set_seat_vars, set_session_params, set_session_vars,
    set_xdg_common_paths,
};
use post_login::{EnvironmentStartError, PostLoginEnvironment};

use env_container::EnvironmentContainer;
use utmpx::add_utmpx_entry;

use log::info;

#[derive(Debug, Clone)]
pub struct LemursConfig {
    pub do_log: bool,
    pub shell_login_flag: ShellLoginFlag,

    pub client_log_path: String,
    pub xserver_log_path: String,

    pub xserver_timeout_secs: u16,
    pub pam_service: String,

    pub tty: u8,

    pub x11_display: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellLoginFlag {
    None,
    Short,
    Long,
}

pub struct Hooks<'a> {
    pub pre_validate: Option<&'a dyn Fn()>,
    pub pre_auth: Option<&'a dyn Fn()>,
    pub pre_environment: Option<&'a dyn Fn()>,
    pub pre_wait: Option<&'a dyn Fn()>,
    pub pre_return: Option<&'a dyn Fn()>,
}

pub enum StartSessionError {
    AuthenticationError(AuthenticationError),
    EnvironmentStartError(EnvironmentStartError),
}

impl From<EnvironmentStartError> for StartSessionError {
    fn from(value: EnvironmentStartError) -> Self {
        Self::EnvironmentStartError(value)
    }
}

impl From<AuthenticationError> for StartSessionError {
    fn from(value: AuthenticationError) -> Self {
        Self::AuthenticationError(value)
    }
}

pub fn start_session(
    username: &str,
    password: &str,
    post_login_env: &PostLoginEnvironment,
    hooks: &Hooks<'_>,
    config: &LemursConfig,
) -> Result<(), StartSessionError> {
    info!(
        "Starting new session for '{}' in environment '{:?}'",
        username, post_login_env
    );

    if let Some(pre_validate_hook) = hooks.pre_validate {
        pre_validate_hook();
    }

    let mut process_env = EnvironmentContainer::take_snapshot();

    if let Some(pre_auth_hook) = hooks.pre_auth {
        pre_auth_hook();
    }

    if matches!(post_login_env, PostLoginEnvironment::X { .. }) {
        set_display(&config.x11_display, &mut process_env);
    }
    set_session_params(&mut process_env, post_login_env);

    let auth_session = try_auth(username, password, &config.pam_service)?;

    if let Some(pre_environment_hook) = hooks.pre_environment {
        pre_environment_hook();
    }

    let tty = config.tty;
    let uid = auth_session.uid;
    let homedir = &auth_session.dir;
    let shell = &auth_session.shell;

    set_seat_vars(&mut process_env, tty);
    set_session_vars(&mut process_env, uid);
    set_basic_variables(&mut process_env, username, homedir, shell);
    set_xdg_common_paths(&mut process_env, homedir);

    let spawned_environment = post_login_env.spawn(&auth_session, &mut process_env, &config)?;

    let pid = spawned_environment.pid();

    let utmpx_session = add_utmpx_entry(username, tty, pid);
    drop(process_env);

    info!("Waiting for environment to terminate");

    if let Some(pre_wait_hook) = hooks.pre_wait {
        pre_wait_hook();
    }

    spawned_environment.wait();

    info!("Environment terminated. Returning to Lemurs...");

    if let Some(pre_return_hook) = hooks.pre_return {
        pre_return_hook();
    }

    drop(utmpx_session);
    drop(auth_session);

    Ok(())
}
