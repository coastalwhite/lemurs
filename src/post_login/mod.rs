use log::{error, info, warn};
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::path::Path;

use users::get_user_groups;

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

use crate::auth::AuthUserInfo;
use crate::config::{Config, ShellLoginFlag};
use crate::env_container::EnvironmentContainer;
use crate::post_login::x::setup_x;

use nix::unistd::{Gid, Uid};

use self::wait_with_log::LemursChild;
use self::x::XSetupError;

pub(crate) mod env_variables;
mod wait_with_log;
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
            Self::XSetup(err) => write!(f, "Failed to setup X11 server. Reason: '{err}'"),
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
        .unwrap_or_else(|| {
            error!("Failed to get user groups. This should not happen here...");
            std::process::exit(1);
        })
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
    X11 {
        server: LemursChild,
        client: LemursChild,
    },
    Wayland(LemursChild),
    Tty(Child),
}

impl SpawnedEnvironment {
    pub fn pid(&self) -> u32 {
        match self {
            Self::X11 { client, .. } | Self::Wayland(client) => client.id(),
            Self::Tty(client) => client.id(),
        }
    }

    pub fn wait(self) {
        info!("Waiting for client to exit");

        match self {
            Self::X11 {
                mut client,
                mut server,
            } => {
                match client.wait() {
                    Ok(exit_code) => info!("Client exited with exit code `{exit_code}`"),
                    Err(err) => {
                        error!("Failed to wait for client. Reason: {err}");
                    }
                };

                info!("Telling X server to shut down");
                match server.send_sigterm() {
                    Ok(_) => {}
                    Err(err) => error!("Failed to terminate X11. Reason: {err}"),
                }

                info!("Waiting for X server");
                match server.wait() {
                    Ok(_) => {}
                    Err(err) => error!("Failed to wait for X11. Reason: {err}"),
                }
            }
            Self::Wayland(mut client) => match client.wait() {
                Ok(exit_code) => info!("Client exited with exit code `{exit_code}`"),
                Err(err) => error!("Failed to wait for client. Reason: {err}"),
            },
            Self::Tty(mut client) => match client.wait() {
                Ok(exit_code) => info!("Client exited with exit code `{exit_code}`"),
                Err(err) => error!("Failed to wait for client. Reason: {err}"),
            },
        }
    }
}

impl PostLoginEnvironment {
    pub fn spawn(
        &self,
        user_info: &AuthUserInfo<'_>,
        process_env: &mut EnvironmentContainer,
        config: &Config,
    ) -> Result<SpawnedEnvironment, EnvironmentStartError> {
        let shell_login_flag = match config.shell_login_flag {
            ShellLoginFlag::None => None,
            ShellLoginFlag::Short => Some("-l"),
            ShellLoginFlag::Long => Some("--login"),
        };

        let mut client = lower_command_permissions_to_user(Command::new(SYSTEM_SHELL), user_info);

        let log_path = config.do_log.then_some(Path::new(&config.client_log_path));

        if let Some(shell_login_flag) = shell_login_flag {
            client.arg(shell_login_flag);
        }

        client.arg("-c");

        match self {
            PostLoginEnvironment::X { xinitrc_path } => {
                info!("Starting X11 session");

                let server = setup_x(process_env, user_info, config)
                    .map_err(EnvironmentStartError::XSetup)?;

                client.arg(format!("{} {}", "/etc/lemurs/xsetup.sh", xinitrc_path));

                let client = match LemursChild::spawn(client, log_path) {
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

                client.arg(script_path);

                let child = match LemursChild::spawn(client, log_path) {
                    Ok(child) => child,
                    Err(err) => {
                        error!("Failed to start Wayland Compositor. Reason '{err}'");
                        return Err(EnvironmentStartError::WaylandStart);
                    }
                };

                Ok(SpawnedEnvironment::Wayland(child))
            }
            PostLoginEnvironment::Shell => {
                info!("Starting TTY shell");

                let shell = &user_info.shell;
                let child = match client
                    .arg(shell)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .stdin(Stdio::inherit())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(err) => {
                        error!("Failed to start TTY shell. Reason '{err}'");
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
                            "'{file_name}' is not executable and therefore not added as an environment",
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
