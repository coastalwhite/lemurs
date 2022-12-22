use chvt_rs::chvt;
use log::{error, info, warn};
use std::path::PathBuf;

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::auth::SessionUser;
use crate::{can_run, RunError};
use crate::session_environment::wayland::WaylandStartContext;
use crate::session_environment::x11::X11StartContext;
use env_variables::{init_environment, set_xdg_env};

use self::wayland::WaylandStartError;
use self::x11::X11StartError;

mod env_variables;
mod wayland;
mod x11;

const SYSTEM_SHELL: &str = "sh";

#[derive(Clone)]
pub struct SessionInitializer {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EnvironmentContext<'a> {
    pub system_shell: &'a str,
    pub session_tty: u8,
    pub x_bin_path: &'a str,
    pub display: &'a str,
    pub virtual_terminal: &'a str,
}

impl Default for EnvironmentContext<'static> {
    fn default() -> Self {
        Self {
            session_tty: 2,
            system_shell: SYSTEM_SHELL,
            x_bin_path: "X",
            display: ":1",
            virtual_terminal: "vt01",
        }
    }
}

impl SessionInitializer {
    /// Turn a [`SessionInitializer`] into a [`SessionEnvironment:X11`].
    #[inline]
    pub fn as_x11_env(self) -> SessionEnvironment {
        SessionEnvironment::X11(self)
    }

    /// Turn a [`SessionInitializer`] into a [`SessionEnvironment::Wayland`].
    #[inline]
    pub fn as_wayland_env(self) -> SessionEnvironment {
        SessionEnvironment::Wayland(self)
    }
}

#[derive(Clone)]
pub enum SessionEnvironment {
    X11(SessionInitializer),
    Wayland(SessionInitializer),
    Shell,
}

pub enum EnvironmentStartError {
    Run(RunError),
    InitializerFailed,
    InitializerWaitFailed,
    StdErrNonUtf8,
    WaylandStart(WaylandStartError),
    X11Start(X11StartError),
}

impl SessionEnvironment {
    pub fn start(
        &self,
        session_user: &SessionUser,
    ) -> Result<(), EnvironmentStartError> {
        let context = EnvironmentContext::default();
        self.start_with_context(session_user, &context)
    }

    pub fn start_with_context<'a>(
        &self,
        session_user: &SessionUser,
        context: &EnvironmentContext<'a>,
    ) -> Result<(), EnvironmentStartError> {
        let result = self.internal_start_with_context(session_user, context);

        info!("Switch back to Lemurs virtual terminal");

        // TODO: Make this work with the configuration
        if unsafe { chvt(2) }.is_err() {
            warn!("Failed to switch back to Lemurs virtual terminal");
        }

        result
    }

    fn internal_start_with_context<'a>(
        &self,
        session_user: &SessionUser,
        context: &EnvironmentContext<'a>,
    ) -> Result<(), EnvironmentStartError> {
        can_run()?;

        let uid = session_user.user_id();
        let gid = session_user.group_id();
        let groups = session_user.groups().to_owned();

        init_environment(
            session_user.username(),
            session_user.home_dir(),
            session_user.shell(),
        );
        info!("Set environment variables");

        set_xdg_env(uid, session_user.home_dir(), context.session_tty, self);
        info!("Set XDG environment variables");

        let mut initializer = match self {
            SessionEnvironment::X11(initializer) => {
                let context = X11StartContext::from(context);
                let mut initializer = initializer
                    .start_x11(&session_user, &context)
                    .map_err(EnvironmentStartError::X11Start)?;

                // Pipe the stdout and stderr to us so we can read it.
                initializer.stdout(Stdio::piped()).stderr(Stdio::piped());

                initializer
            }
            SessionEnvironment::Wayland(initializer) => {
                let context = WaylandStartContext::from(context);
                let mut initializer = initializer
                    .start_wayland(&session_user, &context)
                    .map_err(EnvironmentStartError::WaylandStart)?;

                // Pipe the stdout and stderr to us so we can read it.
                initializer.stdout(Stdio::piped()).stderr(Stdio::piped());

                initializer
            }
            SessionEnvironment::Shell => {
                info!("Starting TTY shell");

                let shell = &session_user.shell();

                let mut initializer = Command::new(shell);

                initializer
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .stdin(Stdio::inherit());

                initializer
            }
        };


        // Lower the permissions of the initializer process
        let to_session_user_env = move || {
            use nix::unistd::{setgid, setgroups, setuid};

            // NOTE: The order here is very vital, otherwise permission errors occur
            // This is basically a copy of how the nightly standard library does it.
            setgroups(&groups)?;
            setgid(gid)?;
            setuid(uid)?;

            Ok(())
        };
        unsafe { initializer.pre_exec(to_session_user_env) };

        // Actually spawn the initializer process
        let initializer = match initializer.spawn() {
            Ok(cmd) => cmd,
            Err(err) => {
                error!("Failed to start initializer. Reason '{}'", err);
                return Err(EnvironmentStartError::InitializerFailed);
            }
        };

        // Update the UTMPX session to include the initializer pid
        session_user.set_pid(initializer.id());

        // Wait for the session to end
        let output = match initializer.wait_with_output() {
            Ok(output) => output,
            Err(err) => {
                error!("Failed to wait for environment to exit, Reason: '{}'", err);
                return Err(EnvironmentStartError::InitializerWaitFailed);
            }
        };

        // Print the stdout if it is at all available
        match std::str::from_utf8(&output.stdout) {
            Ok(output) if !output.trim().is_empty() => {
                info!("Environment's stdout: \"\"\"\n{}\n\"\"\"", output.trim());
            }
            Err(err) => {
                warn!("Failed to read STDOUT output as UTF-8. Reason: '{}'", err);
            }
            Ok(_) => {}
        };

        // Return the `stderr` if the child process did not exit correctly.
        if !output.status.success() {
            warn!("Environment came back with non-zero exit code.");

            match std::str::from_utf8(&output.stderr) {
                Ok(output) if !output.trim().is_empty() => {
                    warn!("Environment's stderr: \"\"\"\n{}\n\"\"\"", output.trim());
                }
                Err(err) => {
                    warn!("Failed to read STDERR output as UTF-8. Reason: '{}'", err);
                    return Err(EnvironmentStartError::StdErrNonUtf8);
                }
                Ok(_) => {}
            };
        }


        info!("Ended session");

        Ok(())
    }
}

pub fn get_envs(with_tty_shell: bool) -> Vec<SessionEnvironment> {
    let x11_envs = x11::get_envs();
    let wayland_envs = wayland::get_envs();

    let envs_len = 0;
    let envs_len = envs_len + x11_envs.len();
    let envs_len = envs_len + wayland_envs.len();
    let envs_len = envs_len + usize::from(with_tty_shell);

    let mut envs = Vec::with_capacity(envs_len);

    for x11_env in x11_envs.into_iter() {
        envs.push(SessionEnvironment::X11(x11_env));
    }

    for wayland_env in wayland_envs.into_iter() {
        envs.push(SessionEnvironment::Wayland(wayland_env));
    }

    if with_tty_shell {
        envs.push(SessionEnvironment::Shell);
    }

    envs
}

impl From<RunError> for EnvironmentStartError {
    fn from(value: RunError) -> Self {
        EnvironmentStartError::Run(value)
    }
}
