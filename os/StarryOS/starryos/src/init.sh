#!/bin/sh

export HOME=/root
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export DEBIAN_FRONTEND=noninteractive

printf '\033[96m\033[1mWelcome to Starry OS!\033[0m\n'
env
echo

printf 'Use \033[1m\033[3mapt\033[0m to install packages.\n'
echo

cd ~

# Use bash if available, otherwise fall back to sh
exec /bin/bash -l
