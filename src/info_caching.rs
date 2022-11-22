use lazy_static::lazy_static;
use log::{info, warn};
use regex::Regex;
use std::fs::{read_to_string, write};

pub const CACHE_PATH: &str = "/var/cache/lemurs";
const USERNAME_REGEX_STR: &str = r"^[a-z][-a-z0-9]*$";
const USERNAME_LENGTH_LIMIT: usize = 32;

// Saved in the /var/cache/lemurs file as
// ```
// ENVIRONMENT\n
// USERNAME
// ```
#[derive(Debug, Clone)]
pub struct CachedInfo {
    environment: Option<String>,
    username: Option<String>,
}

impl CachedInfo {
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

lazy_static! {
    static ref USERNAME_REGEX: Regex = Regex::new(USERNAME_REGEX_STR).unwrap();
}

pub fn get_cached_information() -> CachedInfo {
    info!(
        "Attempting to get a cached information from '{}'",
        CACHE_PATH
    );

    match read_to_string(CACHE_PATH) {
        Ok(cached) => {
            // Remove any line feeds
            let cached = cached.trim().to_string();

            let mut lines = cached.lines();

            let cached_environment = lines.next();
            let cached_username = lines.next();

            info!(
                "Read cache file and found environment '{}' and username '{}'",
                cached_environment.unwrap_or("None"),
                cached_username.unwrap_or("None")
            );

            let cached_username = if let Some(cached_username) = cached_username {
                // Username length check
                if cached_username.len() > USERNAME_LENGTH_LIMIT {
                    warn!("Cached username is too long and is therefore not loaded.");
                    None

                // Username validity check (through regex)
                } else if !USERNAME_REGEX.is_match(cached_username) {
                    warn!("Cached username is not a valid username and is therefore not loaded.");
                    None
                } else {
                    Some(cached_username)
                }
            } else {
                cached_username
            };

            CachedInfo {
                environment: cached_environment.map(|x| x.to_string()),
                username: cached_username.map(|x| x.to_string()),
            }
        }
        Err(err) => {
            warn!("Unable to read cache file. Reason: '{}'", err);
            CachedInfo {
                environment: None,
                username: None,
            }
        }
    }
}

pub fn set_cache(environment: Option<&str>, username: Option<&str>) {
    info!("Attempting to set cache");

    let username = if let Some(username) = username {
        // Username length check
        if username.len() > USERNAME_LENGTH_LIMIT {
            warn!("Username is too long and is therefore not cached.");
            return;
        }

        // Username validity check (through regex)
        if !USERNAME_REGEX.is_match(username) {
            warn!("Username is not a valid username and is therefore not cached.");
            None
        } else {
            Some(username)
        }
    } else {
        None
    };

    let cache_file_content = format!(
        "{}\n{}\n",
        environment.unwrap_or_default(),
        username.unwrap_or_default()
    );

    match write(CACHE_PATH, cache_file_content) {
        Err(err) => {
            warn!("Failed to set username to cache file. Reason: '{}'", err);
        }
        _ => {
            info!("Successfully set username in cache file");
        }
    }
}
