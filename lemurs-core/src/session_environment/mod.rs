use log::{error, info, warn};
use std::path::PathBuf;

use users::get_user_groups;

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::utmpx::add_utmpx_entry;
use crate::auth::AuthUserInfo;
use env_variables::{init_environment, set_xdg_env};

use nix::unistd::{Gid, Uid};

mod env_variables;
mod wayland;
mod x11;

const SYSTEM_SHELL: &str = "/bin/sh";

#[derive(Clone)]
pub struct SessionScript {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone)]
pub enum SessionEnvironment {
    X11(SessionScript),
    Wayland(SessionScript),
    Shell,
}

pub enum EnvironmentStartError {
    WaylandStart,
    XSetup(x11::XSetupError),
    XStartEnv,
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

fn lower_command_permissions_to_user(
    mut command: Command,
    user_info: &AuthUserInfo<'_>,
) -> Command {
    let uid = user_info.uid;
    let gid = user_info.gid;
    let groups: Vec<Gid> = get_user_groups(&user_info.name, gid)
        .unwrap()
        .iter()
        .map(|group| Gid::from_raw(group.gid()))
        .collect();

    unsafe {
        command.pre_exec(move || {
            // NOTE: The order here is very vital, otherwise permission errors occur
            // This is basically a copy of how the nightly standard library does it.
            nix::unistd::setgroups(&groups)
                .and(nix::unistd::setgid(Gid::from_raw(gid)))
                .and(nix::unistd::setuid(Uid::from_raw(uid)))
                .map_err(|err| err.into())
        });
    }

    command
}

impl SessionEnvironment {
    pub fn start<'a>(
        &self,
        session_tty: u8,
        user_info: &AuthUserInfo<'a>,
    ) -> Result<(), EnvironmentStartError> {
        init_environment(&user_info.name, &user_info.dir, &user_info.shell);
        info!("Set environment variables");

        set_xdg_env(user_info.uid, &user_info.dir, session_tty, self);
        info!("Set XDG environment variables");

        match self {
            SessionEnvironment::X11(SessionScript { name, path }) => {
                info!("Starting X11 session '{}'", name);
                x11::setup_x(user_info).map_err(EnvironmentStartError::XSetup)?;
                let child =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("-c")
                        .arg(format!("{} {}", "/etc/lemurs/xsetup.sh", path.display()))
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(err) => {
                            error!("Failed to start X11 environment. Reason '{}'", err);
                            return Err(EnvironmentStartError::XStartEnv);
                        }
                    };

                let pid = child.id();
                let session = add_utmpx_entry(&user_info.name, session_tty, pid);

                wait_for_child_and_log(child);
                drop(session);
            }
            SessionEnvironment::Wayland(SessionScript { name, path }) => {
                info!("Starting Wayland session '{}'", name);
                let child =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("-c")
                        .arg(path)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(err) => {
                            error!("Failed to start Wayland Compositor. Reason '{}'", err);
                            return Err(EnvironmentStartError::WaylandStart);
                        }
                    };

                let pid = child.id();
                let session = add_utmpx_entry(&user_info.name, session_tty, pid);
                wait_for_child_and_log(child);
                drop(session);
            }
            SessionEnvironment::Shell => {
                info!("Starting TTY shell");
                let shell = &user_info.shell;
                let child = match lower_command_permissions_to_user(Command::new(shell), user_info)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .stdin(Stdio::inherit())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(err) => {
                        error!("Failed to start TTY shell. Reason '{}'", err);
                        return Ok(());
                    }
                };

                let pid = child.id();
                let session = add_utmpx_entry(&user_info.name, session_tty, pid);
                wait_for_child_and_log(child);
                drop(session);
            }
        }

        Ok(())
    }
}

pub fn get_envs(with_tty_shell: bool) -> Vec<SessionEnvironment> {
    let x11_envs = x11::get_envs();
    let wayland_envs = wayland::get_envs();

    let envs_len = 0;
    let envs_len = envs_len + x11_envs.len();
    let envs_len = envs_len + wayland_envs.len();
    let envs_len = envs_len + if with_tty_shell { 1 } else { 0 };

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
