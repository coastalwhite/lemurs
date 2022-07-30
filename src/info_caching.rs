use lazy_static::lazy_static;
use log::{info, warn};
use regex::Regex;
use std::fs::{read_to_string, write};

const USERNAME_CACHE_PATH: &str = "/var/cache/lemurs";
const USERNAME_REGEX_STR: &str = r"^[a-z][-a-z0-9]*$";
const USERNAME_LENGTH_LIMIT: usize = 32;
const USERNAME_DISPLAY_SIZE: usize = 32;

lazy_static! {
    static ref USERNAME_REGEX: Regex = Regex::new(USERNAME_REGEX_STR).unwrap();
}

pub fn get_cached_username() -> Option<String> {
    info!(
        "Attempting to get a cached username from '{}'",
        USERNAME_CACHE_PATH
    );

    match read_to_string(USERNAME_CACHE_PATH) {
        Ok(cached_username) => {
            // Remove any line feeds
            let cached_username = cached_username.trim().to_string();

            // Truncate the length of the username that is displayed within the logs
            let displayed_username = if cached_username.len() > USERNAME_DISPLAY_SIZE {
                &cached_username[..USERNAME_DISPLAY_SIZE]
            } else {
                &cached_username
            };

            info!("Read cache file and found '{}'", displayed_username);

            // Username length check
            if cached_username.len() > USERNAME_LENGTH_LIMIT {
                warn!("Cached username is too long and is therefore not loaded.");
                return None;
            }

            // Username validity check (through regex)
            if !USERNAME_REGEX.is_match(&cached_username) {
                warn!("Cached username is not a valid username and is therefore not loaded.");
                return None;
            }

            Some(cached_username)
        }
        Err(err) => {
            warn!("Unable to read cache file. Reason: '{}'", err);
            None
        }
    }
}

pub fn set_cached_username(username: &str) {
    // Truncate the length of the username that is displayed within the logs
    let displayed_username = if username.len() > USERNAME_DISPLAY_SIZE {
        &username[..USERNAME_DISPLAY_SIZE]
    } else {
        username
    };

    info!(
        "Attempting to set username '{}' to '{}'",
        displayed_username, USERNAME_CACHE_PATH
    );

    // Username length check
    if username.len() > USERNAME_LENGTH_LIMIT {
        warn!("Username is too long and is therefore not cached.");
        return;
    }

    // Username validity check (through regex)
    if !USERNAME_REGEX.is_match(username) {
        warn!("Username is not a valid username and is therefore not cached.");
        return;
    }

    match write(USERNAME_CACHE_PATH, username) {
        Err(err) => {
            warn!("Failed to set username to cache file. Reason: '{}'", err);
        }
        _ => {
            info!("Successfully set username in cache file");
        }
    }
}
