# Internals

## Boot Sequence

As input, it will

### 0. Take snapshot of the environment variables.
1. Set `DISPLAY` environment variable
2. Set `XDG` session parameters.
	1. `XDG_SESSION_CLASS` with `user`
	2. `XDG_SESSION_DESKTOP` with `$DESKTOP` # NOT YET IMPLEMENTED
	3. `XDG_CURRENT_DESKTOP` with `$DESKTOP` # NOT YET IMPLEMENTED
	4. `XDG_SESSION_TYPE` with `tty` / `x11` / `wayland`
4. Call PAM with Username and Password
5. Set `XDG` seat variables. Since `pam_systemd` / `logind` may set these variables, they are only set if they are not set already.
	1. `XDG_SEAT` with `seat0`
	2. `XDG_VTNR` with `$TTY`
6. Set `XDG` session variables. Since `pam_systemd` / `logind` may set these variables, they are only set if they are not set already. The variables and their default values are:
	1. `XDG_RUNTIME_DIR` with `/run/user/$UID`
	2. `XDG_SESSION_ID` with `1`
5. Set basic environment variables
		1. `HOME` with user's home directory in `/etc/passwd`
		2. `PWD` to the user's home directory in `/etc/passwd`
		3. `SHELL` to the user's shell in the `/etc/passwd` file
		4. `USER` to the username
		5. `LOGNAME` to the username
		6. `PATH` to `/usr/local/sbin:/usr/local/bin:/usr/bin`
6. Set `XDG` common paths.
	1. `XDG_CONFIG_DIR` with `$HOME/.config`
	2. `XDG_CACHE_HOME` with `$HOME/.cache`
	3. `XDG_DATA_HOME` with `$HOME/.local/share`
	4. `XDG_STATE_HOME` with `$HOME/.local/state`
	5. `XDG_DATA_DIRS` with `/usr/local/share:/usr/share`
	6. `XDG_CONFIG_DIRS` with `/etc/xdg`
7. Start Session Environment. This can possibly set more environment variables.
8. Log UTMP Entry
10. Restore to previously taken snapshot
11. Wait for Session Environment to Finish
```