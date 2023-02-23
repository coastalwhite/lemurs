use chvt_rs::chvt;
use log::{error, info, warn};
use nix::unistd::{Gid, Uid};
use std::fmt::Display;
use std::path::PathBuf;

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::SessionUser;
use crate::session_environment::wayland::WaylandStartContext;
use crate::session_environment::x11::X11StartContext;
use crate::{can_run, RunError, UserInfo};

use self::wayland::WaylandStartError;
use self::x11::X11StartError;

pub(crate) mod env_variables;
mod wayland;
mod x11;

const SYSTEM_SHELL: &str = "sh";

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum SessionEnvironment {
    X11(SessionInitializer),
    Wayland(SessionInitializer),
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    X11,
    Wayland,
    Shell,
}

pub struct SessionProcess<'a> {
    type_specific_content: SessionProcessContent,
    session_user: Option<SessionUser<'a>>,
}

#[derive(Debug)]
pub enum SessionProcessContent {
    X11 { server: Child, client: Child },
    Wayland(Child),
    Shell(Child),
}

#[derive(Debug)]
pub enum SessionCommand {
    X11 { server: Child, client: Command },
    Wayland(Command),
    Shell(Command),
}

#[derive(Debug, Clone)]
pub enum EnvironmentStartError {
    Run(RunError),
    InitializerFailed,
    InitializerWaitFailed,
    StdErrNonUtf8,
    WaylandStart(WaylandStartError),
    X11Start(X11StartError),
    X11ServerKillFailed,
    ReusedSessionUser,
}

impl Display for SessionInitializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' at '{}'", self.name, self.path.display())
    }
}

impl Display for SessionEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X11(initializer) => write!(f, "X11 session {}", initializer),
            Self::Shell => f.write_str("tty shell"),
            Self::Wayland(initializer) => write!(f, "Wayland session {}", initializer),
        }
    }
}

impl Display for EnvironmentStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Run(err) => write!(f, "Run Error. {err}"),
            Self::InitializerFailed => f.write_str("Failed to create or spawn initializer"),
            Self::InitializerWaitFailed => f.write_str("Failed to wait for initializer"),
            Self::StdErrNonUtf8 => {
                f.write_str("Initializer failed and the stderr is not valid UTF-8.")
            }
            Self::WaylandStart(err) => write!(f, "Wayland Start Error. {err}"),
            Self::X11Start(err) => write!(f, "X11 Start Error. {err}"),
            Self::X11ServerKillFailed => write!(f, "Failed to kill X11 server"),
            Self::ReusedSessionUser => {
                write!(f, "Reused Session User after it already spawned a session")
            }
        }
    }
}

impl SessionType {
    pub(crate) fn as_xdg_type(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::Wayland => "wayland",
            Self::Shell => "tty",
        }
    }
}

impl<'a> SessionProcess<'a> {
    fn as_ref(&self) -> &Child {
        use SessionProcessContent::*;

        match &self.type_specific_content {
            X11 { client, .. } | Wayland(client) | Shell(client) => &client,
        }
    }

    pub fn authenticate(&mut self, session_user: SessionUser<'a>) {
        // Insert the pid into the UTMPX entry, if needed
        session_user.set_pid(self.pid());

        self.session_user = Some(session_user);
    }

    pub fn pid(&self) -> u32 {
        self.as_ref().id()
    }

    fn map_with_cleanup(
        self,
        f: impl Fn(Child) -> Result<(), EnvironmentStartError>,
    ) -> Result<(), EnvironmentStartError> {
        use SessionProcessContent::*;

        match self.type_specific_content {
            X11 { mut server, client } => {
                f(client)?;
                server.kill().map_err(|err| {
                    error!("Failed to kill X11 server, Reason: '{}'", err);
                    EnvironmentStartError::X11ServerKillFailed
                })
            }
            Wayland(client) | Shell(client) => f(client),
        }
    }

    pub fn wait(self) -> Result<(), EnvironmentStartError> {
        info!("Waiting for environment to terminate");

        self.map_with_cleanup(|initializer| {
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

            info!("Environment terminated");

            Ok(())
        })
    }
}

impl<'a> From<SessionProcessContent> for SessionProcess<'a> {
    fn from(type_specific_content: SessionProcessContent) -> Self {
        SessionProcess {
            type_specific_content,
            session_user: None,
        }
    }
}

impl SessionCommand {
    fn as_mut(&mut self) -> &mut Command {
        match self {
            Self::X11 { ref mut client, .. }
            | Self::Wayland(ref mut client)
            | Self::Shell(ref mut client) => client,
        }
    }

    pub fn pipe_output(&mut self) {
        self.as_mut().stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    pub fn inherit_io(&mut self) {
        self.as_mut()
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit());
    }

    pub fn lower_permission_pre_exec(&mut self, uid: Uid, gid: Gid, groups: Vec<Gid>) {
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
        unsafe { self.as_mut().pre_exec(to_session_user_env) };
    }

    pub fn spawn<'a>(self) -> io::Result<SessionProcess<'a>> {
        Ok(SessionProcess {
            type_specific_content: match self {
                Self::X11 { server, mut client } => SessionProcessContent::X11 {
                    server,
                    client: client.spawn()?,
                },
                Self::Wayland(mut client) => SessionProcessContent::Wayland(client.spawn()?),
                Self::Shell(mut client) => SessionProcessContent::Shell(client.spawn()?),
            },
            session_user: None,
        })
    }
}

impl SessionEnvironment {
    pub fn session_type(&self) -> SessionType {
        match self {
            Self::X11(..) => SessionType::X11,
            Self::Wayland(..) => SessionType::Wayland,
            Self::Shell => SessionType::Shell,
        }
    }

    pub fn spawn(&self, session_user: &UserInfo) -> Result<SessionProcess, EnvironmentStartError> {
        let context = EnvironmentContext::default();
        self.spawn_with_context(session_user, &context)
    }

    pub fn spawn_with_context<'a>(
        &self,
        session_user: &UserInfo,
        context: &EnvironmentContext<'a>,
    ) -> Result<SessionProcess, EnvironmentStartError> {
        let result = self.internal_spawn_with_context(session_user, context);

        info!("Switch back to Lemurs virtual terminal");

        // TODO: Make this work with the configuration
        if unsafe { chvt(2) }.is_err() {
            warn!("Failed to switch back to Lemurs virtual terminal");
        }

        result
    }

    fn internal_spawn_with_context<'a>(
        &self,
        user_info: &UserInfo,
        context: &EnvironmentContext<'a>,
    ) -> Result<SessionProcess, EnvironmentStartError> {
        can_run()?;

        let uid = user_info.user_id();
        let gid = user_info.group_id();
        let groups = user_info.groups().to_owned();

        let mut session_command = match self {
            SessionEnvironment::X11(initializer) => {
                let context = X11StartContext::from(context);
                let mut session_command = initializer
                    .start_x11(user_info, &context)
                    .map_err(EnvironmentStartError::X11Start)?;

                // Pipe the stdout and stderr to us so we can read it.
                session_command.pipe_output();

                session_command
            }
            SessionEnvironment::Wayland(initializer) => {
                let context = WaylandStartContext::from(context);
                let mut session_command = initializer
                    .start_wayland(user_info, &context)
                    .map_err(EnvironmentStartError::WaylandStart)?;

                // Pipe the stdout and stderr to us so we can read it.
                session_command.pipe_output();

                session_command
            }
            SessionEnvironment::Shell => {
                info!("Starting TTY shell");

                let shell = &user_info.shell();

                let mut session_command = SessionCommand::Shell(Command::new(shell));

                session_command.inherit_io();

                session_command
            }
        };

        session_command.lower_permission_pre_exec(uid, gid, groups);

        // Actually spawn the initializer process
        match session_command.spawn() {
            Ok(cmd) => Ok(cmd.into()),
            Err(err) => {
                error!("Failed to start initializer. Reason '{}'", err);
                Err(EnvironmentStartError::InitializerFailed)
            }
        }
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
