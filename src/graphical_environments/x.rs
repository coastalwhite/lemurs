use nix::unistd::{initgroups, Uid, Gid, setuid, setgid};
use rand::Rng;
use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::{thread, time};

use std::fs::File;
use std::path::{Path, PathBuf};

use log::{error, info};

use super::GraphicalEnvironment;

use pgs_files::passwd::PasswdEntry;

const DISPLAY: &str = ":1";
const VIRTUAL_TERMINAL: &str = "vt01";

const SYSTEM_SHELL: &str = "/bin/sh";

pub fn mcookie() -> String {
    let mut rng = rand::thread_rng();

    let cookie: u128 = rng.gen();
    format!("{:032x}", cookie)
}

pub struct X {
    child: Option<Child>,
}

impl X {
    pub fn new() -> Self {
        Self { child: None }
    }
}

impl GraphicalEnvironment for X {
    fn start(&mut self, passwd_entry: &PasswdEntry) -> io::Result<()> {
        if self.child.is_some() {
            // TODO: Replace this with an error
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
            .status()
            .unwrap(); // TODO: Remove unwrap

        info!("Xauth setup");
        info!("Starting X server");

        // Start the X server
        self.child = Some(
            Command::new(SYSTEM_SHELL)
                .arg("-c")
                .arg(format!("/usr/bin/X {} {}", DISPLAY, VIRTUAL_TERMINAL))
                .spawn()?,
        );

        info!("X server is booted!!!");

        // Wait for XServer to boot-up
        // TODO: There should be a better way of doing this.
        thread::sleep(time::Duration::from_secs(1));

        Ok(())
    }

    fn desktop(&self, script: PathBuf, passwd_entry: &PasswdEntry, _groups: &[u32]) {
        // Init environment for current TTY
        crate::pam::init_environment(&passwd_entry);

        let uid = Uid::from_raw(passwd_entry.uid);
        let gid = Gid::from_raw(passwd_entry.gid);
        initgroups(
            std::ffi::CString::new(passwd_entry.name.clone())
                .unwrap()
                .as_c_str(),
            gid,
        )
        .unwrap();
        setgid(gid).unwrap();
        setuid(uid).unwrap();

        let mut child = Command::new(SYSTEM_SHELL)
            .arg("-c")
            .arg(format!(
                "{} {}",
                "/etc/lemurs/xsetup.sh",
                script.to_str().unwrap()
            ))
            .uid(passwd_entry.uid)
            .gid(passwd_entry.gid)
            // .groups(groups)
            .spawn()
            .unwrap(); // TODO: Remove unwrap

        child.wait().unwrap(); // TODO: Remove unwrap
    }

    fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            child.kill().unwrap(); // TODO: Remove unwrap
        }
    }
}
