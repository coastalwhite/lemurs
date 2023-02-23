use std::error::Error;
use std::fmt::Display;

use log::info;
use pam::{Authenticator, PasswordConv};

use crate::auth::{AuthError, AuthSession, SessionAuthError};

use super::AuthContext;

pub struct PamSession<'a>(Authenticator<'a, PasswordConv>);

#[derive(Debug, Clone)]
pub struct PamContext {
    service: String,
}

/// All the different errors that can occur during PAM opening an authenticated session
#[derive(Debug, Clone)]
pub enum PamError {
    InvalidPamService(String),
    SessionOpen,
}

impl Default for PamContext {
    fn default() -> Self {
        Self {
            service: "lemurs".to_string(),
        }
    }
}

impl<'a> AuthSession for PamSession<'a> {
    type Err = PamError;
    type Context = PamContext;

    fn open_with_context(
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        context: &Self::Context,
    ) -> Result<Self, SessionAuthError> {
        let username = username.as_ref();
        let password = password.as_ref();

        info!("Started opening session");

        let mut authenticator = Authenticator::with_password(&context.service)
            .map_err(|_| PamError::InvalidPamService(context.service.to_string()))?;

        info!("Gotten Authenticator");

        // Authenticate the user
        authenticator
            .get_handler()
            .set_credentials(username, password);

        info!("Got handler");

        // Validate the account
        authenticator
            .authenticate()
            .map_err(|_| AuthError::InvalidCredentials)?;

        info!("Validated account");

        authenticator
            .open_session()
            .map_err(|_| PamError::SessionOpen)?;

        info!("Opened session");

        // NOTE: Logout happens automatically here with `drop` of session and context
        Ok(PamSession(authenticator))
    }
}

impl Display for PamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PamError::*;

        match self {
            InvalidPamService(service) => write!(
                f,
                "Failed to create authenticator with PAM service '{}'",
                service
            ),
            SessionOpen => f.write_str("Failed to open a PAM session"),
        }
    }
}

impl Error for PamError {}

impl AuthContext {
    pub fn pam_service(mut self, service: impl ToString) -> Self {
        self.backend_specific.service = service.to_string();
        self
    }
}
