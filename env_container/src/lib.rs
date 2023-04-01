use std::collections::HashMap;
use std::env;

use log::{error, info};

/// The `EnvironmentContainer` is abstract the process environment and allows for restoring to an
/// earlier state
#[derive(Debug)]
pub struct EnvironmentContainer {
    snapshot: HashMap<String, String>,
    snapshot_pwd: String,
    owned: HashMap<&'static str, String>,
}

impl EnvironmentContainer {
    /// Take a snapshot of the current state of the Environment
    pub fn take_snapshot() -> Self {
        let pwd = match env::current_dir().map(|pathbuf| pathbuf.to_str().map(str::to_string)) {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => {
                error!("Could not find the working directory when taking snapshot");
                String::from("/")
            }
        };

        Self {
            snapshot: env::vars().collect::<HashMap<String, String>>(),
            owned: HashMap::default(),
            snapshot_pwd: pwd,
        }
    }

    /// Set an environment variable and own the value
    ///
    /// This function will overwrite a value that is currently in the environment or that is
    /// currently owned.
    pub fn set(&mut self, key: &'static str, value: impl Into<String>) {
        let value = value.into();

        env::set_var(key, &value);
        info!("Set environment variable '{}' to '{}'", key, value);

        self.owned.insert(key, value);
    }

    /// Set an environment variable if it is not already set
    ///
    /// If the variable was already set, then the [`EnvironmentContainer`] considers the value as
    /// one of its own.
    pub fn set_or_own(&mut self, key: &'static str, value: impl Into<String>) {
        if let Ok(value) = env::var(key) {
            info!(
                "Skipped setting environment variable '{}'. It was already set to '{}'",
                key, value
            );
            self.owned.insert(key, value);
        } else {
            self.set(key, value)
        }
    }

    /// Sets the working directory
    pub fn set_current_dir(&mut self, value: impl Into<String>) {
        let value = value.into();

        if env::set_current_dir(&value).is_ok() {
            info!("Successfully changed working directory to {}!", value);
        } else {
            error!("Failed to change the working directory to {}", value);
        }
        self.snapshot_pwd = value;
    }
}

// When a EnvironmentContainer is dropped it restores all the set variables to the previous state.
impl Drop for EnvironmentContainer {
    fn drop(&mut self) {
        // Remove all owned variables for which we have an accurate environment value
        info!("Removing session environment variables");
        for (key, value) in self.owned.iter() {
            if env::var(key).as_ref() == Ok(value) {
                env::remove_var(key);
            }
        }

        // Restore all snapshot values for which disappeared
        info!("Reverting to environment before session");
        for (key, value) in self.snapshot.iter() {
            if env::var(key).is_err() {
                env::set_var(key, value);
            }
        }

        info!("Reverting to working directory before session");
        if env::set_current_dir(&self.snapshot_pwd).is_err() {
            error!(
                "Failed to change the working directory back to {}",
                &self.snapshot_pwd
            );
        }
    }
}
