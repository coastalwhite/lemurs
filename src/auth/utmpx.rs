use std::time::SystemTime;

use libc::{utmpx, USER_PROCESS, DEAD_PROCESS};
use log::{error, info};

pub struct UtmpxSession(utmpx);

pub fn add_utmpx_entry(username: &str, tty: u8, pid: u32) -> UtmpxSession {
    info!("Adding UTMPX record");

    // https://man7.org/linux/man-pages/man0/utmpx.h.0p.html
    // https://github.com/fairyglade/ly/blob/master/src/login.c
    let entry = {
        let mut s: utmpx = unsafe { std::mem::zeroed() };

        s.ut_type = USER_PROCESS;
        s.ut_pid = pid as libc::pid_t;

        let mut ut_user = [0; 32];
        for (i, b) in username.as_bytes().into_iter().take(32).enumerate() {
            ut_user[i] = *b as i8;
        }
        s.ut_user = ut_user;

        if tty > 12 {
            error!("Invalid TTY");
            std::process::exit(1);
        }
        let tty_c_char = (b'0' + tty) as i8;

        let mut ut_line = [0; 32];
        ut_line[0] = b't' as i8;
        ut_line[1] = b't' as i8;
        ut_line[2] = b'y' as i8;
        ut_line[3] = tty_c_char;
        s.ut_line = ut_line;

        let mut ut_id = [0; 4];
        ut_id[0] = tty_c_char;
        s.ut_id = ut_id;

        let epoch_duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| {
                error!("Invalid System Time");
                std::process::exit(1);
            })
            .as_micros();

        s.ut_tv.tv_sec = (epoch_duration / 1_000_000).try_into().unwrap_or_else(|_| {
            error!("Invalid System Time (TV_SEC Overflow)");
            std::process::exit(1);
        });
        s.ut_tv.tv_usec = (epoch_duration % 1_000_000).try_into().unwrap_or_else(|_| {
            error!("Invalid System Time (TV_USEC Overflow)");
            std::process::exit(1);
        });

        s
    };

    unsafe { libc::setutxent() };
    unsafe { libc::pututxline(&entry as *const utmpx) };

    info!("Added UTMPX record");

    UtmpxSession(entry)
}

impl Drop for UtmpxSession {
    fn drop(&mut self) {
        let UtmpxSession(mut entry) = self;

        info!("Removing UTMPX record");

        entry.ut_type = DEAD_PROCESS;

        entry.ut_line = [0; 32];
        entry.ut_user = [0; 32]; 

        entry.ut_tv.tv_usec = 0;
        entry.ut_tv.tv_sec = 0;

        unsafe {
            libc::setutxent();
            libc::pututxline(&entry as *const utmpx);
            libc::endutxent();
        }
    }
    
}