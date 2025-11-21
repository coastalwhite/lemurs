#!/usr/bin/env sh

ROOT_CMD=''

fail() {
    echo "$*"
    exit 1
}

inst() {
    if ! "$ROOT_CMD" install -m644 -CT "$1" "$2"; then
        fail "installing [$1] to [$2] failed"
    fi
}

if [ "$(id --user)" -eq 0 ]; then
    fail 'do not run as root'
elif ! which install > /dev/null 2>&1; then
    fail '[install] utility needs to be installed'
fi

if which doas > /dev/null 2>&1; then
    ROOT_CMD='doas'
elif which sudo > /dev/null 2>&1; then
    ROOT_CMD='sudo'
else
    fail '[sudo] or [doas] needs to be installed'
fi

echo 'Lemurs install script'
echo

# Compile lemurs
echo 'Compile Lemurs'
cargo build --release || exit 1

# Move lemurs to /usr/bin
echo 'Move lemurs into /usr/bin'
inst 'target/release/lemurs' '/usr/bin/lemurs'

# Create lemurs directory
echo 'Create lemurs configuration directory'
echo 'NOTE: You still have to move your X or Wayland startup into the proper directories'
"$ROOT_CMD" mkdir -p '/etc/lemurs/wms' || exit 1
"$ROOT_CMD" mkdir -p '/etc/lemurs/wayland' || exit 1

# Copy over configuration file
echo 'Copy over default configuration'
inst 'extra/config.toml' '/etc/lemurs/config.toml'

# Copy over xsetup
echo 'Copy over more files'
inst 'extra/xsetup.sh' '/etc/lemurs/xsetup.sh'

# Copy over default xinitrc
[ -z "$XINITRC" ] && XINITRC="$HOME/.xinitrc"

if [ -s "$XINITRC" ]; then
    echo 'Copy over existing xinitrc'
    inst "$XINITRC" '/etc/lemurs/wms/xinitrc'
fi

# Copy over PAM service
echo 'Copy over PAM service'
inst 'extra/lemurs.pam' '/etc/pam.d/lemurs'

# Cache the current user
echo 'Caching the current user'
echo "xinitrc\n$USER" | "$ROOT_CMD" tee /var/cache/lemurs > /dev/null || exit 1

# Disable previous Display Manager
echo 'Disabling the current display-manager. This might throw an error if no display manager is set up.'
"$ROOT_CMD" systemctl disable display-manager.service || echo "display-manager.service not enabled"

# Copy over systemd service
echo 'Setting up lemurs service'
inst 'extra/lemurs.service' '/usr/lib/systemd/system/lemurs.service'

# Enable lemurs
echo 'Enable the lemurs service'
"$ROOT_CMD" systemctl enable lemurs.service || exit 1

exit 0
