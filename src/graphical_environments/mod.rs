use std::io;
use std::path::PathBuf;

use pgs_files::passwd::PasswdEntry;

pub trait GraphicalEnvironment {
    fn start(&mut self, passwd_entry: &PasswdEntry) -> io::Result<()>;
    fn desktop(&self, script: PathBuf, passwd_entry: &PasswdEntry, groups: &[u32]);
    fn stop(&mut self);
}

mod x;
pub use x::X;
