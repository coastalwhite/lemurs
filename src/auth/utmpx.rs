use std::time::SystemTime;

use libc::{c_char, utmpx};
use log::{error, info};

pub struct UtmpxSession(utmpx);

pub fn add_utmpx_entry(username: &str, tty: u8, pid: u32) -> UtmpxSession {
    info!("Adding UTMPX record");

    // Check the MAN page for utmp for more information
    // `man utmp`
    //
    // https://man7.org/linux/man-pages/man0/utmpx.h.0p.html
    // https://github.com/fairyglade/ly/blob/master/src/login.c
    let entry = {
        let mut s: utmpx = unsafe { std::mem::zeroed() };

        // ut_line    --- Device name of tty - "/dev/"
        // ut_id      --- Terminal name suffix
        // ut_user    --- Username
        // ut_host    --- Hostname for remote login, or kernel version for run-level messages
        // ut_exit    --- Exit status of a process marked as DEAD_PROCESS; not used by Linux init(1)
        // ut_session --- Session ID (getsid(2)) used for windowing
        // ut_tv {    --- Time entry was made
        //     tv_sec     --- Seconds
        //     tv_usec    --- Microseconds
        // }
        // ut_addr_v6 --- Internet address of remote

        s.ut_type = libc::USER_PROCESS;
        s.ut_pid = pid as libc::pid_t;

        for (i, b) in username.as_bytes().iter().take(32).enumerate() {
            s.ut_user[i] = *b as c_char;
        }

        if tty > 12 {
            error!("Invalid TTY");
            std::process::exit(1);
        }
        let tty_c_char = (b'0' + tty) as c_char;

        s.ut_line[0] = b't' as c_char;
        s.ut_line[1] = b't' as c_char;
        s.ut_line[2] = b'y' as c_char;
        s.ut_line[3] = tty_c_char;

        s.ut_id[0] = tty_c_char;

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

    unsafe {
        libc::setutxent();
        libc::pututxline(&entry as *const utmpx);
    };

    info!("Added UTMPX record");

    UtmpxSession(entry)
}

impl Drop for UtmpxSession {
    fn drop(&mut self) {
        let UtmpxSession(mut entry) = self;

        info!("Removing UTMPX record");

        entry.ut_type = libc::DEAD_PROCESS;

        entry.ut_line = <[c_char; 32]>::default();
        entry.ut_user = <[c_char; 32]>::default();

        entry.ut_tv.tv_usec = 0;
        entry.ut_tv.tv_sec = 0;

        unsafe {
            libc::setutxent();
            libc::pututxline(&entry as *const utmpx);
            libc::endutxent();
        }
    }
}
