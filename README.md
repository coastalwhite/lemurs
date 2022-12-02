<div align="center">
	
# Lemurs 🐒
A TUI Display/Login Manager written in Rust
	
</div>

![Cover image](./cover.png)

> Note: the project is installable and working, but there might still be some
> limitations.

A minimal TUI [Display Manager/Login
Manager](https://wiki.archlinux.org/title/Display_manager) written in Rust
similar to [Ly](https://github.com/nullgemm/ly).

## Goal

The goal of this project is to create a small, robust and yet customizable
Login Manager which can serve as the front-end to your graphical GNU/Linux.
Lemurs uses Linux PAM as its method of authentication.

## Installation

There are two different ways to install Lemurs. Both require the rust toolchain
to be installed. I.e. there is currently no precompiled option.

### Arch Linux --- AUR

Lemurs can be installed from the [AUR](https://aur.archlinux.org). This will
build the package on your local machine. It will automatically pull in rustup,
but you might have to set the default toolchain with `rustup default stable`.

```bash
paru -S lemurs-git # paru can be replaced by any other AUR helper
```

or

```bash
git clone https://aur.archlinux.org/lemurs-git.git
cd lemurs-git
makepkg -si
```

### Compiling from source

The `install.sh` script can be used to compile and setup the display manager on
your Unix machine. This will perform multiple steps:

1. Build the project in release mode (requires Rust's _cargo_)
2. Setup the `/etc/lemurs` folder which contains some of the configuration and
   necessary files such as your selection of window managers.
3. Disables the previous Display Manager
4. Copies over the _systemd_ service and enables it.

Although you might first want to set up some window managers (see
[Usage](#Usage)), upon rebooting you should now see Lemurs.

## Usage

After installation you can add your environments by creating runnable scripts.

For your Xorg put your [xinitrc](https://wiki.archlinux.org/title/Xinit) scripts
in the `/etc/lemurs/wms` directory. For Wayland, put a script that starts your
compositor in the `/etc/lemurs/wayland` directory. For both cases, the name of
the runnable script file is the name that is shown in the environment switcher
within lemurs. Multiple Xorg and Wayland environments can exist at the same time.

### Example 1: BSPWM

For the [bspwm](https://github.com/baskerville/bspwm) window manager, you might
add the script `/etc/lemurs/wms/bspwm`.

```bash
#! /bin/sh
sxhkd &
exec bspwm
```

Remember to make this script runnable. This is done with the `chmod +x
/etc/lemurs/wms/bspwm` command.

Upon rebooting your new `bspwm` should show up within Lemurs.

### Example 2: Sway

For the [sway](https://swaywm.org/) compositor and window manager, you might
add the script `/etc/lemurs/wayland/sway`. Ensure that you have sway installed
and added yourself to the `seat` group.

```bash
#! /bin/sh
exec sway
```

Remember to make this script runnable. This is done with the `chmod +x
/etc/lemurs/wayland/sway` command.

Upon rebooting your new `sway` should show up within Lemurs.

## Configuration

Many parts for the UI can be configured with the `/etc/lemurs/config.toml`
file. This file contains all the options and explanations of their purpose.
The flag `--config <CONFIG FIlE>` can be used to select another configuration
file instead. An example configuration can be found in the `extra` folder in
this repository.

## Preview & Debugging

Lemurs logs a lot of information of it running to a logging file. This is
located by default at `/var/log/lemurs.log`, but can be turned of by running
with the `--no-log` flag.

If you want to test your configuration file you can also run `lemurs
--preview`. This will run a preview instance of your configuration. This will
automatically create a `lemurs.log` in the working directory.

## File Structure

Below is overview of the source files in this project and a short description of
each of them and their use. This can be used by people who want to contribute or
want to tweak details for their own installation.

```
|- src: Rust Source Code
|  |- main.rs: CLI argument parsing & main logic
|  |- auth: Interaction with PAM modules
|  |- config.rs: Configuration file format and options
|  |- info_caching.rs: Reading and writing cached login information
|  |- post_login: All logic after authentication
|  |  |- env_variables.rs: General environment variables settings
|  |  |- x.rs: Logic concerning Xorg
|  |- ui: TUI code
|  |  |- mod.rs: UI calling logic, separated over 2 threads
|  |  |- input_field.rs: TUI input field used for username and password
|  |  |- power_menu.rs: Shutdown and Reboot options UI
|  |  |- status_message.rs: UI for error and information messages
|  |  |- switcher.rs: UI for environment switcher
|  |  |- chunks.rs: Division of the TUI screen
|- extra: Configuration and extra files needed
|  |- config.toml: The default configuration file
|  |- xsetup.sh: Script used to setup a Xorg session
|  |- lemurs.service: The systemd service used to start at boot
```

## Platforms

Tested on

- ArchLinux (Vanilla, ArcoLinux)
- VoidLinux
- Ubuntu (make sure to install `libpam-dev`)

## License

The project is made available under the MIT and APACHE license. See the
`LICENSE-MIT` and `LICENSE-APACHE` files, respectively, for more information.

## Contributions

Please report any bugs and possible improvements as an issue within this
repository. Pull requests are also welcome.
