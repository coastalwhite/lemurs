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
echo 'Step 1: Compile Lemurs'
cargo +nightly build --release 
if [ $? -ne 0 ]; then exit 1; fi

# Move lemurs to /usr/bin
echo 'Step 2: Move lemurs into /usr/bin'
$ROOT_CMD cp -f "target/release/lemurs" "/usr/bin/lemurs"
if [ $? -ne 0 ]; then exit 1; fi

# Create lemurs directory
echo 'Step 3: Create lemurs configuration directory'
$ROOT_CMD mkdir -p "/etc/lemurs/wms"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over configuration file
echo 'Step 4: Copy over default configuration'
$ROOT_CMD cp -f "extra/config.toml" "/etc/lemurs/config.toml"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over xsetup
echo 'Step 5: Copy over more files'
$ROOT_CMD cp -f "extra/xsetup.sh" "/etc/lemurs/xsetup.sh"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over default xinitrc
if [ -f .xinitrc ]
then
    echo 'Step 6: Copy over existing xinitrc'
	$ROOT_CMD cp -f "~/.xinitrc" "/etc/lemurs/wms/xinitrc"
fi

# Cache the current user
echo 'Step 7: Caching the current user'
$ROOT_CMD /bin/bash -c "echo $USER > /var/cache/lemurs"

# Disable previous Display Manager
echo 'Step 7: Disabling the current display-manager. This might throw an error if no display manager is set up.'
$ROOT_CMD systemctl disable display-manager.service

# Copy over systemd service
echo 'Step 8: Setting up lemurs service'
$ROOT_CMD cp -f extra/lemurs.service /usr/lib/systemd/system/lemurs.service
if [ $? -ne 0 ]; then exit 1; fi

# Enable lemurs
echo 'Step 9: Enable the lemurs service'
$ROOT_CMD systemctl enable lemurs.service
if [ $? -ne 0 ]; then exit 1; fi

# Make sure Xauthority file exists
echo 'Step 10: Ensure the Xauthority file exists'
touch ~/.Xauthority
if [ $? -ne 0 ]; then exit 1; fi
