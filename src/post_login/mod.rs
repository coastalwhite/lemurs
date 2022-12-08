use log::{error, info, warn};
use std::fs;

use users::get_user_groups;

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::utmpx::add_utmpx_entry;
use crate::auth::AuthUserInfo;
use crate::config::Config;
use env_variables::{init_environment, set_xdg_env};

use nix::unistd::{Gid, Uid};

mod env_variables;
mod x;

const SYSTEM_SHELL: &str = "/bin/sh";

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";
const WAYLAND_FOLDER_PATH: &str = "/etc/lemurs/wayland";

#[derive(Clone)]
pub enum PostLoginEnvironment {
    X { xinitrc_path: String },
    Wayland { script_path: String },
    Shell,
}

pub enum EnvironmentStartError {
    WaylandStart,
    XSetup(x::XSetupError),
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

impl PostLoginEnvironment {
    pub fn start<'a>(
        &self,
        config: &Config,
        user_info: &AuthUserInfo<'a>,
    ) -> Result<(), EnvironmentStartError> {
        init_environment(&user_info.name, &user_info.dir, &user_info.shell);
        info!("Set environment variables");

        set_xdg_env(user_info.uid, &user_info.dir, config.tty, self);
        info!("Set XDG environment variables");

        match self {
            PostLoginEnvironment::X { xinitrc_path } => {
                x::setup_x(user_info).map_err(EnvironmentStartError::XSetup)?;
                let child =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("-c")
                        .arg(format!("{} {}", "/etc/lemurs/xsetup.sh", xinitrc_path))
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
                let session = add_utmpx_entry(&user_info.name, config.tty, pid);

                wait_for_child_and_log(child);
                drop(session);
            }
            PostLoginEnvironment::Wayland { script_path } => {
                info!("Starting Wayland Session");
                let child =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("-c")
                        .arg(script_path)
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
                let session = add_utmpx_entry(&user_info.name, config.tty, pid);
                wait_for_child_and_log(child);
                drop(session);
            }
            PostLoginEnvironment::Shell => {
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
                let session = add_utmpx_entry(&user_info.name, config.tty, pid);
                wait_for_child_and_log(child);
                drop(session);
            }
        }

        Ok(())
    }
}

pub fn get_envs(with_tty_shell: bool) -> Vec<(String, PostLoginEnvironment)> {
    // NOTE: Maybe we can do something smart with `with_capacity` here.
    let mut envs = Vec::new();

    match fs::read_dir(INITRCS_FOLDER_PATH) {
        Ok(paths) => {
            for path in paths {
                if let Ok(path) = path {
                    let file_name = path.file_name().into_string();

                    if let Ok(file_name) = file_name {
                        if let Ok(metadata) = path.metadata() {
                            if std::os::unix::fs::MetadataExt::mode(&metadata) & 0o111 == 0 {
                                warn!(
                            "'{}' is not executable and therefore not added as an environment",
                            file_name
                        );

                                continue;
                            }
                        }

                        envs.push((
                            file_name,
                            PostLoginEnvironment::X {
                                xinitrc_path: match path.path().to_str() {
                                    Some(p) => p.to_string(),
                                    None => {
                                        warn!(
                                    "Skipped item because it was impossible to convert to string"
                                );
                                        continue;
                                    }
                                },
                            },
                        ));
                    } else {
                        warn!("Unable to convert OSString to String");
                    }
                } else {
                    warn!("Ignored errorinous path: '{}'", path.unwrap_err());
                }
            }
        }
        Err(_) => {
            warn!("Failed to read from the X folder '{}'", INITRCS_FOLDER_PATH);
        }
    }

    match fs::read_dir(WAYLAND_FOLDER_PATH) {
        Ok(paths) => {
            for path in paths {
                if let Ok(path) = path {
                    let file_name = path.file_name().into_string();

                    if let Ok(file_name) = file_name {
                        if let Ok(metadata) = path.metadata() {
                            if std::os::unix::fs::MetadataExt::mode(&metadata) & 0o111 == 0 {
                                warn!(
                            "'{}' is not executable and therefore not added as an environment",
                            file_name
                        );

                                continue;
                            }
                        }

                        envs.push((
                            file_name,
                            PostLoginEnvironment::Wayland {
                                script_path: match path.path().to_str() {
                                    Some(p) => p.to_string(),
                                    None => {
                                        warn!(
                                    "Skipped item because it was impossible to convert to string"
                                );
                                        continue;
                                    }
                                },
                            },
                        ));
                    } else {
                        warn!("Unable to convert OSString to String");
                    }
                } else {
                    warn!("Ignored errorinous path: '{}'", path.unwrap_err());
                }
            }
        }
        Err(_) => {
            warn!(
                "Failed to read from the wayland folder '{}'",
                WAYLAND_FOLDER_PATH
            );
        }
    }

    if envs.is_empty() || with_tty_shell {
        envs.push(("TTYSHELL".to_string(), PostLoginEnvironment::Shell));
    }

    envs
}
