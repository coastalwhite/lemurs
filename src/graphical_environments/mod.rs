use std::error::Error;
use std::path::PathBuf;

use pgs_files::passwd::PasswdEntry;

pub trait GraphicalEnvironment {
    fn start(&mut self, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>>;
    fn desktop(&self, script: PathBuf, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>>;
    fn stop(&mut self);
}

mod x;
pub use x::X;
