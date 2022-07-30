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

The `install.sh` script can be used to compile and setup the display manager on
your Unix machine. This will perform multiple steps:

1. Build the project in release mode (requires Rust's _cargo_)
2. Setup the `/etc/lemurs` folder which contains some of the configuration and
   necessary files such as your selection of window managers.
3. Disables the previous Display Manager
4. Copy over the _systemd_ service and enables it.

Although you might first want to set up some window managers (see
[Usage](#Usage)), upon rebooting you should now see Lemurs.

## Usage

After running the installation script you can add your window managers by
creating runnable scripts also known as
[xinitrc](https://wiki.archlinux.org/title/Xinit)s under the `/etc/lemurs/wms`
folders. The name of the script is used as the name within lemurs. For example,
for the [bspwm](https://github.com/baskerville/bspwm) window manager, you might
add the script `/etc/lemurs/wms/bspwm`.

```bash
#! /bin/sh

sxhkd &
exec bspwm
```

Remember to make this script runnable. This is done with the `chmod +x
/etc/lemurs/wms/bspwm` command.

Upon rebooting your new _xinitrc_ should show up within Lemurs.

## Configuration

Many parts for the UI can be configured with the `/etc/lemurs/config.toml`
file. This file contains all the options and explanations of their purpose.
The flag `--config <CONFIG FIlE>` can be used to select another configuration
file instead. An example configuration can be found in the `/extra` folder.

## License

The project is made available under the MIT and APACHE license. See the
`LICENSE-MIT` and `LICENSE-APACHE` files, respectively, for more information.

## Debugging / Logging

Lemurs logs a lot of information of it running to a logging file. This is
located by default at `/var/log/lemurs.log`, but can be turned of by running
with the `--nolog` flag.

If you want to test your configuration file you can also run `lemurs
--preview`. This will run a preview instance of your configuration. This will
automatically create a `lemurs.log` in the working directory.

## Contributions

Please report any bugs and possible improvements as an issue within this
repository. Pull requests are also welcome.
