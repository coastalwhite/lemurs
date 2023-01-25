use log::{error, info, warn};
use std::error::Error;
use std::fmt::Display;
use std::fs;

use users::get_user_groups;

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::AuthUserInfo;
use crate::config::Config;
use crate::env_container::EnvironmentContainer;
use crate::post_login::x::setup_x;

use nix::unistd::{Gid, Uid};

use self::x::XSetupError;

pub(crate) mod env_variables;
mod x;

const SYSTEM_SHELL: &str = "/bin/sh";

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";
const WAYLAND_FOLDER_PATH: &str = "/etc/lemurs/wayland";

#[derive(Debug, Clone)]
pub enum PostLoginEnvironment {
    X { xinitrc_path: String },
    Wayland { script_path: String },
    Shell,
}

impl PostLoginEnvironment {
    pub fn to_xdg_type(&self) -> &'static str {
        match self {
            Self::Shell => "tty",
            Self::X { .. } => "x11",
            Self::Wayland { .. } => "wayland",
        }
    }

    // pub fn to_xdg_desktop(&self) -> &str {
    //     // TODO: Implement properly
    //     ""
    // }
}

#[derive(Debug, Clone)]
pub enum EnvironmentStartError {
    WaylandStart,
    XSetup(XSetupError),
    XStartEnv,
    TTYStart,
}

impl Display for EnvironmentStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WaylandStart => f.write_str("Failed to start Wayland compositor"),
            Self::XSetup(err) => write!(f, "Failed to setup X11 server. Reason: '{}'", err),
            Self::XStartEnv => f.write_str("Failed to start X11 client"),
            Self::TTYStart => f.write_str("Failed to start TTY"),
        }
    }
}

impl Error for EnvironmentStartError {}
impl From<XSetupError> for EnvironmentStartError {
    fn from(value: XSetupError) -> Self {
        Self::XSetup(value)
    }
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

pub enum SpawnedEnvironment {
    X11 { server: Child, client: Child },
    Wayland(Child),
    Tty(Child),
}

impl SpawnedEnvironment {
    pub fn pid(&self) -> u32 {
        match self {
            Self::X11 { client, .. } | Self::Wayland(client) | Self::Tty(client) => client.id(),
        }
    }

    pub fn wait(self) {
        let child = match self {
            Self::X11 { client, .. } | Self::Wayland(client) | Self::Tty(client) => client,
        };

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
                }
            };
        }
    }
}

impl PostLoginEnvironment {
    pub fn spawn<'a>(
        &self,
        user_info: &AuthUserInfo<'a>,
        process_env: &mut EnvironmentContainer,
        _config: &Config,
    ) -> Result<SpawnedEnvironment, EnvironmentStartError> {
        match self {
            PostLoginEnvironment::X { xinitrc_path } => {
                info!("Starting X11 session");
                let server =
                    setup_x(process_env, user_info).map_err(EnvironmentStartError::XSetup)?;
                let client =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("--login")
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

                Ok(SpawnedEnvironment::X11 { server, client })
            }
            PostLoginEnvironment::Wayland { script_path } => {
                info!("Starting Wayland session");
                let child =
                    match lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info)
                        .arg("--login")
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

                Ok(SpawnedEnvironment::Wayland(child))
            }
            PostLoginEnvironment::Shell => {
                info!("Starting TTY shell");

                let shell = &user_info.shell;
                // TODO: Instead of calling the shell directly we should be calling it through
                // `/bin/bash --login`
                let child = match lower_command_permissions_to_user(Command::new(shell), user_info)
                    .arg("--login")
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .stdin(Stdio::inherit())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(err) => {
                        error!("Failed to start TTY shell. Reason '{}'", err);
                        return Err(EnvironmentStartError::TTYStart);
                    }
                };

                Ok(SpawnedEnvironment::Tty(child))
            }
        }
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
