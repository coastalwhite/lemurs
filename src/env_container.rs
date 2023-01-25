use std::collections::HashMap;
use std::env;

use log::info;

/// The `EnvironmentContainer` is abstract the process environment and allows for restoring to an
/// earlier state
#[derive(Debug)]
pub struct EnvironmentContainer {
    snapshot: HashMap<String, String>,
    owned: HashMap<&'static str, String>,
}

impl EnvironmentContainer {
    pub fn take_snapshot() -> Self {
        Self {
            snapshot: env::vars().collect::<HashMap<String, String>>(),
            owned: HashMap::default(),
        }
    }

    /// Set an environment variable and own the value
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

    pub fn restore(self) {
        // Remove all owned variables for which we have an accurate environment value
        for (key, value) in self.owned.into_iter() {
            if env::var(key) == Ok(value) {
                env::remove_var(key);
            }
        }

        // Restore all snapshot values for which disappeared
        for (key, value) in self.snapshot.into_iter() {
            if env::var(&key).is_err() {
                env::set_var(key, value);
            }
        }
    }
}
