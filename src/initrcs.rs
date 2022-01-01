use crate::ui::WindowManager;

use std::fs;

use log::warn;

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";

pub fn get_window_managers() -> Vec<WindowManager> {
    let found_paths = match fs::read_dir(INITRCS_FOLDER_PATH) {
        Ok(paths) => paths,
        Err(_) => return Vec::new(),
    };

    // NOTE: Maybe we can do something smart with `with_capacity` here.
    let mut wms = Vec::new();

    // TODO: Maybe this can be done better.
    for path in found_paths {
        if let Ok(path) = path {
            let file_name = path.file_name().into_string();

            if let Ok(file_name) = file_name {
                wms.push(WindowManager::new(
                    file_name,
                    path.path(),
                ));
            } else {
                warn!("Unable to convert OSString to String");
            }
        } else {
            warn!("Ignored errorinous path: '{}'", path.unwrap_err());
        }
    }

    wms
}
