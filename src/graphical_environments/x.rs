use nix::unistd::{initgroups, setgid, setuid, Gid, Uid};
use rand::Rng;
use std::env;
use std::error::Error;
use std::process::{Child, Command, Stdio};
use std::{thread, time};

use std::fs::File;
use std::path::PathBuf;

use log::{error, info};

use super::GraphicalEnvironment;

use pgs_files::passwd::PasswdEntry;

const DISPLAY: &str = ":1";
const VIRTUAL_TERMINAL: &str = "vt01";

const SYSTEM_SHELL: &str = "/bin/sh";

fn mcookie() -> String {
    // TODO: Verify that this is actually safe. Maybe just use the mcookie binary?? Is that always
    // available?
    let mut rng = rand::thread_rng();
    let cookie: u128 = rng.gen();
    format!("{:032x}", cookie)
}

/// The X graphical environment
pub struct X {
    /// This is the instance of X that is running. We need to save that in order to kill it
    child: Option<Child>,
}

impl X {
    pub fn new() -> Self {
        Self { child: None }
    }
}

impl GraphicalEnvironment for X {
    fn start(&mut self, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>> {
        if self.child.is_some() {
            // TODO: Properly handle this situation
            error!("Server already started");
            panic!("Server already started");
        }

        info!("Setup Xauth");

        // Setup xauth
        let xauth_dir =
            PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or(passwd_entry.dir.to_string()));
        let xauth_path = xauth_dir.join(".Xauthority");
        env::set_var("XAUTHORITY", xauth_path.clone());
        env::set_var("DISPLAY", DISPLAY);

        File::create(xauth_path).unwrap();

        Command::new(SYSTEM_SHELL)
            .arg("-c")
            .arg(format!("/usr/bin/xauth add {} . {}", DISPLAY, mcookie()))
            .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
            .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
            .status()
            .unwrap(); // TODO: Remove unwrap

        info!("Xauth setup");
        info!("Starting X server");

        // Start the X server
        self.child = Some(
            Command::new(SYSTEM_SHELL)
                .arg("-c")
                .arg(format!("/usr/bin/X {} {}", DISPLAY, VIRTUAL_TERMINAL))
                .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
                .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
                .spawn()?,
        );

        info!("X server is booted!!!");

        // Wait for XServer to boot-up
        // TODO: There should be a better way of doing this.
        thread::sleep(time::Duration::from_secs(1));

        Ok(())
    }

    fn desktop(&self, script: PathBuf, passwd_entry: &PasswdEntry) -> Result<(), Box<dyn Error>> {
        let uid = Uid::from_raw(passwd_entry.uid);
        let gid = Gid::from_raw(passwd_entry.gid);

        initgroups(
            std::ffi::CString::new(passwd_entry.name.clone())?.as_c_str(),
            gid,
        )?;
        setgid(gid)?;
        setuid(uid)?;

        let mut child = Command::new(SYSTEM_SHELL)
            .arg("-c")
            .arg(format!(
                "{} {}",
                "/etc/lemurs/xsetup.sh",
                script.to_str().unwrap()
            ))
            .stdout(Stdio::null()) // TODO: Maybe this should be logged or something?
            .stderr(Stdio::null()) // TODO: Maybe this should be logged or something?
            .spawn()?;

        child.wait()?;

        Ok(())
    }

    fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            child.kill().unwrap(); // TODO: Remove unwrap
        }
    }
}
