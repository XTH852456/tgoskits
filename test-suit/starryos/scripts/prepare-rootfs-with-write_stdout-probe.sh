#!/usr/bin/sh
set -eu
exec "$(cd "$(dirname "$0")" && pwd)/prepare-rootfs-with-probe.sh" write_stdout
