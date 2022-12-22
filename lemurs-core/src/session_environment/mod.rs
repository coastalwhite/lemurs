use chvt_rs::chvt;
use log::{error, info, warn};
use std::path::PathBuf;

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::SessionUser;
use crate::session_environment::wayland::WaylandStartContext;
use crate::session_environment::x11::X11StartContext;
use env_variables::{init_environment, set_xdg_env};

use self::wayland::WaylandStartError;
use self::x11::X11StartError;

mod env_variables;
mod wayland;
mod x11;

const SYSTEM_SHELL: &str = "/bin/sh";

#[derive(Clone)]
pub struct SessionInitializer {
    pub name: String,
    pub path: PathBuf,
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
    InitializerFailed,
    WaylandStart(WaylandStartError),
    X11Start(X11StartError),
}

fn wait_for_child_and_log(child: Child) {
    let child_output = match child.wait_with_output() {
        Ok(output) => output,
        Err(err) => {
            error!("Failed to wait for environment to exit, Reason: '{}'", err);
            return;
        }
    };

    // Print the stdout if it is at all available
    match std::str::from_utf8(&child_output.stdout) {
        Ok(output) => {
            if !output.trim().is_empty() {
                info!("Environment's stdout: \"\"\"\n{}\n\"\"\"", output.trim());
            }
        }
        Err(err) => {
            warn!("Failed to read STDOUT output as UTF-8. Reason: '{}'", err);
        }
    };

    // Return the `stderr` if the child process did not exit correctly.
    if !child_output.status.success() {
        warn!("Environment came back with non-zero exit code.");

        match std::str::from_utf8(&child_output.stderr) {
            Ok(output) => {
                if !output.trim().is_empty() {
                    warn!("Environment's stderr: \"\"\"\n{}\n\"\"\"", output.trim());
                }
            }
            Err(err) => {
                warn!("Failed to read STDERR output as UTF-8. Reason: '{}'", err);
                return;
            }
        };
    }

    info!("Returning to Lemurs...");
}

impl SessionEnvironment {
    pub fn start(
        &self,
        session_tty: u8,
        session_user: &SessionUser,
    ) -> Result<(), EnvironmentStartError> {
        let uid = session_user.user_id();
        let gid = session_user.group_id();
        let groups = session_user.groups().to_owned();

        init_environment(
            session_user.username(),
            session_user.home_dir(),
            session_user.shell(),
        );
        info!("Set environment variables");

        set_xdg_env(uid, session_user.home_dir(), session_tty, self);
        info!("Set XDG environment variables");

        let mut initializer = match self {
            SessionEnvironment::X11(initializer) => {
                let context = X11StartContext::default();
                initializer
                    .start_x11(&session_user, &context)
                    .map_err(EnvironmentStartError::X11Start)?
            }
            SessionEnvironment::Wayland(initializer) => {
                let context = WaylandStartContext::default();
                initializer
                    .start_wayland(&session_user, &context)
                    .map_err(EnvironmentStartError::WaylandStart)?
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
        wait_for_child_and_log(initializer);

        info!("Switch back to Lemurs virtual terminal");
        // TODO: Make this work with the configuration
        if unsafe { chvt(2) }.is_err() {
            warn!("Failed to switch back to Lemurs virtual terminal");
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
