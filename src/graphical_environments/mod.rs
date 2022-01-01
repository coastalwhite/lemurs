use std::error::Error;
use std::path::PathBuf;

use pgs_files::passwd::PasswdEntry;

pub trait GraphicalEnvironment {
    /// Start the graphical environment
    fn start(&mut self, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>>;
    /// Run the desktop environment
    fn desktop(&self, script: PathBuf, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>>;
    /// Stop the graphical environment
    fn stop(&mut self);
}

mod x;
pub use x::X;
