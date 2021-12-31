use crate::ui::WindowManager;
use std::fs;

const INITRCS_FOLDER_PATH: &str = "/etc/lemurs/wms";

pub fn get_window_managers() -> Vec<WindowManager> {
    let wms = match fs::read_dir(INITRCS_FOLDER_PATH) {
        Ok(paths) => paths,
        Err(_) => return Vec::new(),
    };

    wms
        .map(|entry| {
            let entry = entry.unwrap();
            WindowManager::new(
                entry.file_name().into_string().unwrap(), // TODO: Remove unwrap
                entry.path(),
            )
        })
        .collect()
}
