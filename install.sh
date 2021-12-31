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

# Compile lemurs
cargo build --release 
if [ $? -ne 0 ]; then exit 1; fi

# Move lemurs to /usr/bin
$ROOT_CMD cp -f "target/release/lemurs" "/usr/bin/lemurs"
if [ $? -ne 0 ]; then exit 1; fi

# Create lemurs directory
$ROOT_CMD mkdir -p "/etc/lemurs/wms"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over xsetup
$ROOT_CMD cp -f "extra/xsetup.sh" "/etc/lemurs/xsetup.sh"
if [ $? -ne 0 ]; then exit 1; fi

# Copy over default xinitrc
$ROOT_CMD cp -f "~/.xinitrc" "/etc/lemurs/wms/xinitrc"

# Disable previous Display Manager
$ROOT_CMD systemctl disable display-manager.service

# Copy over systemd service
$ROOT_CMD cp -f extra/lemurs.service /usr/lib/systemd/system/lemurs.service
if [ $? -ne 0 ]; then exit 1; fi

# Disable other DM
$ROOT_CMD systemctl enable lemurs.service
if [ $? -ne 0 ]; then exit 1; fi
