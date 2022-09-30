use log::{error, info, warn};
use std::fs;

use users::get_user_groups;

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use crate::auth::AuthUserInfo;
use crate::config::Config;
use env_variables::{init_environment, set_xdg_env};

use nix::unistd::{Gid, Uid};

mod env_variables;
mod x;

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";

#[derive(Clone)]
pub enum PostLoginEnvironment {
    X { xinitrc_path: String },
    // Wayland { script_path: String },
    Shell,
}

pub enum EnvironmentStartError {
    XSetupError(x::XSetupError),
    XStartEnvError(x::XStartEnvError),
    WaitingForEnv,
}

impl PostLoginEnvironment {
    pub fn start<'a>(
        &self,
        config: &Config,
        user_info: &AuthUserInfo<'a>,
    ) -> Result<(), EnvironmentStartError> {
        init_environment(&user_info.name, &user_info.dir, &user_info.shell);
        info!("Set environment variables.");

        set_xdg_env(user_info.uid, &user_info.dir, config.tty);
        info!("Set XDG environment variables");

        match self {
            PostLoginEnvironment::X { xinitrc_path } => {
                x::setup_x(user_info).map_err(EnvironmentStartError::XSetupError)?;
                let mut gui_environment = x::start_env(user_info, xinitrc_path)
                    .map_err(EnvironmentStartError::XStartEnvError)?;

                gui_environment.wait().map_err(|err| {
                    warn!("Failed waiting for GUI Environment. Reason: {}", err);
                    EnvironmentStartError::WaitingForEnv
                })?;
            }
            PostLoginEnvironment::Shell => {
                let uid = user_info.uid;
                let gid = user_info.gid;
                let groups: Vec<Gid> = get_user_groups(&user_info.name, gid)
                    .unwrap()
                    .iter()
                    .map(|group| Gid::from_raw(group.gid()))
                    .collect();

                info!("Starting TTY shell");
                let shell = &user_info.shell;
                let status = unsafe {
                    Command::new(shell).pre_exec(move || {
                        // NOTE: The order here is very vital, otherwise permission errors occur
                        // This is basically a copy of how the nightly standard library does it.
                        nix::unistd::setgroups(&groups)
                            .and(nix::unistd::setgid(Gid::from_raw(gid)))
                            .and(nix::unistd::setuid(Uid::from_raw(uid)))
                            .map_err(|err| err.into())
                    })
                }
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdin(Stdio::inherit())
                .status();

                match status {
                    Ok(_) => info!("Successfully returned from TTY shell"),
                    Err(err) => {
                        error!(
                            "Failed to start TTY shell, Reason: {}. Returning to Lemurs...",
                            err
                        );
                    }
                };
            }
        }

        Ok(())
    }
}

pub fn get_envs(with_tty_shell: bool) -> Vec<(String, PostLoginEnvironment)> {
    let found_paths = match fs::read_dir(INITRCS_FOLDER_PATH) {
        Ok(paths) => paths,
        Err(_) => return Vec::new(),
    };

    // NOTE: Maybe we can do something smart with `with_capacity` here.
    let mut envs = Vec::new();

    // TODO: Add other post login environment methods
    for path in found_paths {
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
                        // TODO: Remove unwrap
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

    if envs.is_empty() || with_tty_shell {
        envs.push(("TTYSHELL".to_string(), PostLoginEnvironment::Shell));
    }

    envs
}
