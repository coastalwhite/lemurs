use log::{info, warn};
use std::fs::{read_to_string, write};

use crate::config::Config;

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

fn verify_username(username: &str) -> bool {
    // REGEX: "^[a-zA-Z][-a-zA-Z0-9]*$"

    if username.len() > USERNAME_LENGTH_LIMIT {
        return false;
    }

    let mut bytes = username.bytes();

    match bytes.next() {
        Some(b'a'..=b'z' | b'A'..=b'Z') => {}
        _ => return false,
    };

    for b in bytes {
        if !matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-') {
            return false;
        }
    }

    true
}

impl CachedInfo {
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

pub fn get_cached_information(config: &Config) -> CachedInfo {
    let cache_path = &config.cache_path;

    info!("Attempting to get a cached information from '{cache_path}'",);

    match read_to_string(cache_path) {
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
                } else if !verify_username(cached_username) {
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

pub fn set_cache(environment: Option<&str>, username: Option<&str>, config: &Config) {
    let cache_path = &config.cache_path;

    info!("Attempting to set cache: {cache_path}");

    let username = if let Some(username) = username {
        // Username length check
        if username.len() > USERNAME_LENGTH_LIMIT {
            warn!("Username is too long and is therefore not cached.");
            return;
        }

        // Username validity check (through regex)
        if !verify_username(username) {
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

    match write(cache_path, cache_file_content) {
        Err(err) => {
            warn!("Failed to set username to cache file. Reason: '{err}'");
        }
        _ => {
            info!("Successfully set username in cache file");
        }
    }
}
