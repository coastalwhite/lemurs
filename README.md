<p align="center">
	<!-- Icon by SVGRepo under CC0. Notice at the end of the file -->
	<img src="./assets/text-icon.svg" height="200px" alt="Lemur Icon by SVGRepo" />
</p>

Lemurs provides a *Terminal User Interface* (TUI) for a [Display/Login
Managers](https://wiki.archlinux.org/title/Display_manager) in Rust for most
GNU/Linux and BSD distributions. It can work both *with or without SystemD*.
Lemurs works on most Unix systems including Linux, FreeBSD and NetBSD.

## Goal

This project creates a small, robust and yet customizable Login Manager which
can serve as the front-end to your TTY, X11 or Wayland sessions. Lemurs uses
[_Pluggable Authentication Modules_][pam] (PAM) as its method of authentication.

## Screenshot

![Cover image](./assets/cover.png)

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/lemurs.svg)](https://repology.org/project/lemurs/versions)

Installation follows three steps.

1. Compile the codebase
2. Copy all files to correct positions
3. Enable `init` process to run `lemurs`

### Arch Linux

Lemurs can be installed from the [AUR](https://aur.archlinux.org/packages/lemurs). This will
build the package on your local machine.

```bash
paru -S lemurs # paru can be replaced by any other AUR helper

# Not needed if do don't have a window manager yet
sudo systemctl disable display-manager.service

sudo systemctl enable lemurs.service
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

Configuration is done through a [TOML] file. By default, Lemurs searches for a
`/etc/lemurs/config.toml` file, but an alternative location can be specified
using the `--config <path/to/config.toml>` flag. The configuration type can
contain any subset of the available options. All the options and explanations
as to what they do can be found in the
[`extra/config.toml`](./extra/config.toml) file. This file also serves as the
default configuration.

Additionally, there is the possibility of variables in the configuration file.
By default, Lemurs searches for a `/etc/lemurs/variables.toml` file, but an
alternative location can be specified using the `--variables
<path/to/variables.toml`. The `variables.toml` file may contain key-value pairs
which can be referenced from within the main `config.toml` file. 

Below shows an example of how the `variables.toml` and `config.toml` file
interact.

```toml
# variables.toml
replacement_char = "+"
show_pw_title = true
password_title = "Password :)"
title_color = "white"

# config.toml
[password_field]
content_replacement_character = "$replacement_char"

[password_field.style]
show_title = "$show_pw_title"
title = "Wow a $password_title"
title_color = "$title_color"
```

This will be interpreted as:

```toml
[password_field]
content_replacement_character = "+"

[password_field.style]
show_title = true
title = "Wow a Password :)"
title_color = "white"
```

## Preview & Debugging

Lemurs logs a lot of information of it running to a logging file. There are 3
log files.

- `/var/log/lemurs.log`: This is the main log file that contains information
  about the control flow and is the first place you should look.
- `/var/log/lemurs.client.log`: This is the `stdout` and `stderr` of your
  environment. This can help you debug your scripts in the `/etc/lemurs/wms` or
  `/etc/lemurs/wayland` folder.
- `/var/log/lemurs.xorg.log`: This is the `stdout` and `stderr` of the X server.
  This is only used when you run a X11 based environment.

You can disable logging with the `--no-log` flag. This flag can be adding into
your service manager script if you want to disable logging globally.

If you want to test the UI generated by your configuration file you can also run
`lemurs --preview`. This will run a preview instance of your configuration. This
will automatically create a `lemurs.log` in the working directory.

## File Structure

Below is overview of the source files in this project and a short description of
each of them and their use. This can be used by people who want to contribute or
want to tweak details for their own installation.

```
|- src: Rust Source Code
|  |- main.rs
|  |- chvt.rs: UNIX calls to change of TTY
|  |- cli.rs: CLI argument parsing
|  |- config.rs: Configuration file format and options
|  |- env_container.rs: Handles resetting and resetting the environment variables
|  |- info_caching.rs: Handling cached username and session environment
|  |- auth: Interaction with PAM modules and UTMPX
|  |  |- mod.rs
|  |  |- pam.rs
|  |  |- utmpx.rs
|  |- post_login: All logic after authentication
|  |  |- mod.rs
|  |  |- env_variables.rs: General environment variables settings
|  |  |- x.rs: Logic concerning Xorg
|  |- ui: TUI code
|  |  |- mod.rs: UI calling logic, separated over 2 threads
|  |  |- chunks.rs: Division of the TUI screen
|  |  |- input_field.rs: TUI input field used for username and password
|  |  |- power_menu.rs: Shutdown and Reboot options UI
|  |  |- status_message.rs: UI for error and information messages
|  |  |- switcher.rs: UI for environment switcher
|- extra: Configuration and extra files needed
|  |- config.toml: The default configuration file
|  |- xsetup.sh: Script used to setup a Xorg session
|  |- lemurs.service: The systemd service used to start at boot
|  |- lemurs.pam: PAM service configuration
```

## Shell Keybindings

Lemurs has support for some basic shell keybindings.

- <kbd>Ctrl</kbd> + <kbd>A</kbd>: Go to the beginning of the focused input field
- <kbd>Ctrl</kbd> + <kbd>E</kbd>: Go to the end of the focused input field
- <kbd>Ctrl</kbd> + <kbd>L</kbd>: Clear the focused input field
- <kbd>Ctrl</kbd> + <kbd>U</kbd>: Clear input field text before the cursor
- <kbd>Ctrl</kbd> + <kbd>K</kbd>: Clear input field text after the cursor
- <kbd>Ctrl</kbd> + <kbd>D</kbd>: Same as <kbd>Delete</kbd>
- <kbd>Ctrl</kbd> + <kbd>H</kbd>: Same as <kbd>Backspace</kbd>
- <kbd>Ctrl</kbd> + <kbd>B</kbd>: Same as <kbd>Left</kbd>
- <kbd>Ctrl</kbd> + <kbd>F</kbd>: Same as <kbd>Right</kbd>
- <kbd>Ctrl</kbd> + <kbd>P</kbd>: Same as <kbd>Up</kbd>
- <kbd>Ctrl</kbd> + <kbd>N</kbd>: Same as <kbd>Down</kbd>

## Platforms

Tested on

- ArchLinux (Vanilla, ArcoLinux)
- VoidLinux
- Ubuntu (make sure to install `build-essential` and `libpam-dev`)
- OpenSUSE (make sure to install `pam` and `pam-devel`)

## MSRV Policy

Lemurs has a _Minimum Supported Rust Version_ policy of _N - 2_. This means that
we only use Rust languages features that have been in Rust as of 2 releases.

## License

The icon used at the top of the repository is not a logo and taken as an icon
from the [SVGRepo](https://www.svgrepo.com/svg/252871/lemur). It is marked
under CC0 and therefore freely distributable and amendable under a new
license.

The project is made available under the MIT and APACHE license. See the
`LICENSE-MIT` and `LICENSE-APACHE` files, respectively, for more information.

## Contributions

Please report any bugs and possible improvements as an issue within this
repository. Pull requests are also welcome.

[pam]: https://en.wikipedia.org/wiki/Pluggable_authentication_module
[TOML]: https://toml.io/
