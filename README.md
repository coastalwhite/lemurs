<div align="center">
	
# Lemurs 🐒
A TUI Display/Login Manager written in Rust
	
</div>

![Cover image](./cover.png)

**WIP: Whilst the project is working and installable, there are still a lot of
bugs and limitations.**

A minimal lightweight TUI [Display Manager/Login
Manager](https://wiki.archlinux.org/title/Display_manager) written in Rust
similar to [Ly](https://github.com/nullgemm/ly).

## Goal

The goal of this project is to create a small, robust and yet customizable 
Login Manager which can serve as the front-end to your graphical GNU/Linux
or BSD environment. Lemurs uses Linux PAM as its method of authentication.

## Installation

The `install.sh` script can be used to compile and setup the display manager on
your Unix machine. This will perform multiple steps:

1. Build the project in release mode (requires Rust's *cargo*)
2. Setup the `/etc/lemurs` folder which contains some of the configuration and
   necessary files such as your selection of window managers.
3. Disables the previous Display Manager
4. Copy over the *systemd* service and enables it.
