use log::{info, warn};
use std::fs;

use crate::auth::AuthUserInfo;
use crate::config::Config;
use env_variables::{init_environment, set_xdg_env};

mod env_variables;
mod x;

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";

#[derive(Clone)]
pub enum PostLoginEnvironment {
    X { xinitrc_path: String },
    // Wayland { script_path: String },
    // Shell,
}

pub enum EnvironmentStartError {
    XSetupError(x::XSetupError),
    XStartEnvError(x::XStartEnvError),
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

                gui_environment.wait().unwrap();
            }
        }

        Ok(())
    }
}

pub fn get_envs() -> Vec<(String, PostLoginEnvironment)> {
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
                envs.push((
                    file_name,
                    PostLoginEnvironment::X {
                        // TODO: Remove unwrap
                        xinitrc_path: path.path().to_str().unwrap().to_string(),
                    },
                ));
            } else {
                warn!("Unable to convert OSString to String");
            }
        } else {
            warn!("Ignored errorinous path: '{}'", path.unwrap_err());
        }
    }

    envs
}
