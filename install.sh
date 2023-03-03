#! /bin/sh

ROOT_CMD=""

if which "doas" > /dev/null 2>&1
then
    ROOT_CMD="doas"
else
    if which "sudo" > /dev/null 2>&1
    then
        ROOT_CMD="sudo";
    else
        echo '`sudo` or `doas` needs to be installed';
        exit 1;
    fi
fi

echo 'Lemurs install script'
echo

# Compile lemurs
echo 'Compile Lemurs'
cargo build --release 
if [ $? -ne 0 ]; then exit 1; fi

# Move lemurs to /usr/bin
echo 'Move lemurs into /usr/bin'
$ROOT_CMD cp -f "target/release/lemurs" "/usr/bin/lemurs"
if [ $? -ne 0 ]; then exit 1; fi

# Create lemurs directory
echo 'Create lemurs configuration directory'
echo 'NOTE: You still have to move your X or Wayland startup into the proper directories'
$ROOT_CMD mkdir -p "/etc/lemurs/wms"
$ROOT_CMD mkdir -p "/etc/lemurs/wayland"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over configuration file
echo 'Copy over default configuration'
$ROOT_CMD cp -f "extra/config.toml" "/etc/lemurs/config.toml"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over xsetup
echo 'Copy over more files'
$ROOT_CMD cp -f "extra/xsetup.sh" "/etc/lemurs/xsetup.sh"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over default xinitrc
if [ -f .xinitrc ]
then
    echo 'Copy over existing xinitrc'
	$ROOT_CMD cp -f "~/.xinitrc" "/etc/lemurs/wms/xinitrc"
fi

# Cache the current user
echo 'Copy over PAM service'
$ROOT_CMD cp -f "extra/lemurs.pam" "/etc/pam.d/lemurs"

# Cache the current user
echo 'Caching the current user'
$ROOT_CMD /bin/bash -c "echo $USER > /var/cache/lemurs"

# Disable previous Display Manager
echo 'Disabling the current display-manager. This might throw an error if no display manager is set up.'
$ROOT_CMD systemctl disable display-manager.service

# Copy over systemd service
echo 'Setting up lemurs service'
$ROOT_CMD cp -f extra/lemurs.service /usr/lib/systemd/system/lemurs.service
if [ $? -ne 0 ]; then exit 1; fi

# Enable lemurs
echo 'Enable the lemurs service'
$ROOT_CMD systemctl enable lemurs.service
if [ $? -ne 0 ]; then exit 1; fi

# Make sure Xauthority file exists
echo 'Ensure the Xauthority file exists'
touch ~/.Xauthority
if [ $? -ne 0 ]; then exit 1; fi
