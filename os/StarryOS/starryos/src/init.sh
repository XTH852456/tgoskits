#!/bin/sh

export HOME=/root
export USER=root
export HOSTNAME=starry

printf "Welcome to \033[96m\033[1mStarry OS\033[0m!\n"
env
echo

printf "Use \033[1m\033[3mapk\033[0m to install packages.\n"
echo

# The aarch64 plat-dyn QEMU path does not always have a trustworthy wall clock
# yet, which breaks Alpine's HTTPS certificate validation during early boot.
if [ -f /etc/apk/repositories ]; then
    year="$(date +%Y 2>/dev/null || echo 1970)"
    if [ "$year" -lt 2024 ]; then
        sed -i 's#^https://#http://#' /etc/apk/repositories
    fi
fi

# Do your initialization here!

cd "$HOME" || cd /
export PS1='${USER}@${HOSTNAME}:${PWD} # '
exec /bin/sh -i
