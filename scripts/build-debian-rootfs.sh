#!/usr/bin/env bash
#
# Build a Debian rootfs (ext4 image) for Starry OS using Docker + debootstrap.
#
# Usage:
#   ./scripts/build-debian-rootfs.sh [OPTIONS]
#
# Options:
#   -a, --arch ARCH       Target architecture (default: aarch64)
#   -s, --size SIZE       Image size (default: 2G)
#   -o, --output PATH     Output image path (default: auto-detected from arch)
#   -d, --debian VER      Debian suite (default: bookworm)
#   -p, --password PASS   Root password (default: root)
#   -h, --help            Show this help
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
ARCH="aarch64"
IMAGE_SIZE="2G"
DEBIAN_SUITE="bookworm"
ROOT_PASSWORD="root"
OUTPUT_PATH=""

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# //; s/^#//'
    exit 0
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -a|--arch)       ARCH="$2"; shift 2 ;;
            -s|--size)       IMAGE_SIZE="$2"; shift 2 ;;
            -o|--output)     OUTPUT_PATH="$2"; shift 2 ;;
            -d|--debian)     DEBIAN_SUITE="$2"; shift 2 ;;
            -p|--password)   ROOT_PASSWORD="$2"; shift 2 ;;
            -h|--help)       usage ;;
            *) echo "Unknown option: $1"; usage ;;
        esac
    done
}

resolve_target_and_output() {
    case "$ARCH" in
        aarch64)
            TARGET="aarch64-unknown-none-softfloat"
            DOCKER_ARCH="arm64v8"
            DEB_ARCH="arm64"
            ;;
        riscv64)
            TARGET="riscv64gc-unknown-none-elf"
            DOCKER_ARCH="riscv64"
            DEB_ARCH="riscv64"
            ;;
        x86_64)
            TARGET="x86_64-unknown-none"
            DOCKER_ARCH="amd64"
            DEB_ARCH="amd64"
            ;;
        *)
            echo "Error: unsupported architecture '$ARCH'"
            exit 1
            ;;
    esac

    if [[ -z "$OUTPUT_PATH" ]]; then
        OUTPUT_PATH="$WORKSPACE_ROOT/target/$TARGET/rootfs-$ARCH.img"
    fi
}

check_docker() {
    if ! command -v docker &>/dev/null; then
        echo "Error: docker not found. Please install Docker first."
        exit 1
    fi
    if ! docker info &>/dev/null; then
        echo "Error: Docker daemon is not running."
        exit 1
    fi
}

build_rootfs() {
    local vol_name="starry-rootfs-build-$$"

    echo "==> Building Debian $DEBIAN_SUITE rootfs for $ARCH ($DEB_ARCH)..."
    echo "    Docker image: ${DOCKER_ARCH}/debian:${DEBIAN_SUITE}"
    echo "    Output: $OUTPUT_PATH"
    echo ""

    # Create a named Docker volume to avoid bind-mount nodev/noexec issues
    docker volume create "$vol_name" >/dev/null

    cleanup_volume() {
        docker volume rm "$vol_name" >/dev/null 2>&1 || true
    }
    trap cleanup_volume EXIT

    # Step 1: Run debootstrap + configure rootfs inside Docker using a named volume
    echo "==> [1/2] Running debootstrap and configuring rootfs..."
    docker run --rm \
        --platform "linux/$DEB_ARCH" \
        -v "${vol_name}:/rootfs" \
        "${DOCKER_ARCH}/debian:${DEBIAN_SUITE}" \
        bash -c "
            set -e

            apt-get update
            apt-get install -y debootstrap e2fsprogs busybox-static

            # --- debootstrap ---
            debootstrap --arch=$DEB_ARCH --variant=minbase --no-merged-usr \
                $DEBIAN_SUITE /rootfs http://deb.debian.org/debian

            ROOTFS=/rootfs

            # --- hostname ---
            echo 'starry' > \$ROOTFS/etc/hostname
            echo '127.0.0.1 localhost starry' > \$ROOTFS/etc/hosts

            # --- fstab ---
            cat > \$ROOTFS/etc/fstab <<'FSTAB'
/dev/vda  /  ext4  defaults,noatime  0  1
FSTAB

            # --- set root password ---
            echo 'root:$ROOT_PASSWORD' | chroot \$ROOTFS chpasswd

            # --- install busybox-static and ensure full libc6 (NSS modules) ---
            chroot \$ROOTFS apt-get update
            chroot \$ROOTFS apt-get install -y --reinstall libc6
            chroot \$ROOTFS apt-get install -y busybox-static bash

            # --- ensure /sbin/init is busybox ---
            if [ ! -L \$ROOTFS/sbin/init ] && [ ! -e \$ROOTFS/sbin/init ]; then
                ln -sf /bin/busybox \$ROOTFS/sbin/init
            fi

            # --- inittab for busybox init ---
            cat > \$ROOTFS/etc/inittab <<'INITTAB'
# /etc/inittab - busybox init for Starry OS
::sysinit:/etc/init.d/rcS
::respawn:-/bin/sh
::shutdown:/bin/umount -a -r
INITTAB

            # --- rcS startup ---
            mkdir -p \$ROOTFS/etc/init.d
            cat > \$ROOTFS/etc/init.d/rcS <<'RCS'
#!/bin/sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
mount -t devtmpfs devtmpfs /dev 2>/dev/null
mkdir -p /dev/pts
mount -t devpts devpts /dev/pts 2>/dev/null
hostname starry
export HOME=/root
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RCS
            chmod +x \$ROOTFS/etc/init.d/rcS

            # --- APT config for Starry OS ---
            mkdir -p \$ROOTFS/etc/apt/apt.conf.d
            echo 'APT::Sandbox::User "root";' > \$ROOTFS/etc/apt/apt.conf.d/99no-sandbox
            echo 'APT::Cache-Start "67108864";' > \$ROOTFS/etc/apt/apt.conf.d/99cache-start

            # --- welcome script ---
            mkdir -p \$ROOTFS/root
            cat > \$ROOTFS/root/init.sh <<'INIT_SH'
#!/bin/sh
export HOME=/root
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
echo ''
echo 'Welcome to Starry OS (Debian GNU/Linux)'
echo ''
echo 'Use apt to install packages.'
echo ''
cd ~
sh --login
INIT_SH
            chmod +x \$ROOTFS/root/init.sh

            # --- profile ---
            cat > \$ROOTFS/root/.profile <<'PROFILE'
export PS1='starry:~# '
PROFILE

            # --- network ---
            mkdir -p \$ROOTFS/etc/network
            cat > \$ROOTFS/etc/network/interfaces <<'NETIF'
auto eth0
iface eth0 inet dhcp
NETIF

            # --- clean up ---
            chroot \$ROOTFS apt-get clean
            rm -rf \$ROOTFS/var/lib/apt/lists/*
            rm -rf \$ROOTFS/var/cache/apt/archives/*.deb

            # --- resolv.conf (MUST be after cleanup — Docker overwrites it) ---
            cat > \$ROOTFS/etc/resolv.conf <<'RESOLV'
nameserver 10.0.2.3
nameserver 8.8.8.8
RESOLV
        "

    # Step 2: Create ext4 image inside Docker (no sudo needed on host)
    echo "==> [2/2] Creating ${IMAGE_SIZE} ext4 image..."
    mkdir -p "$(dirname "$OUTPUT_PATH")"

    local output_dir
    output_dir="$(dirname "$OUTPUT_PATH")"
    local output_file
    output_file="$(basename "$OUTPUT_PATH")"

    docker run --rm --privileged \
        --platform "linux/$DEB_ARCH" \
        -v "${vol_name}:/rootfs:ro" \
        -v "${output_dir}":/output \
        "${DOCKER_ARCH}/debian:${DEBIAN_SUITE}" \
        bash -c "
            set -e
            cd /output
            dd if=/dev/zero of=$output_file bs=1 count=0 seek=$IMAGE_SIZE 2>/dev/null
            mkfs.ext4 -F -L starry-rootfs $output_file
            mkdir -p /mnt/rootfs
            mount -o loop $output_file /mnt/rootfs
            cp -a /rootfs/. /mnt/rootfs/
            sync
            umount /mnt/rootfs
            rmdir /mnt/rootfs
        "

    cleanup_volume
    trap - EXIT

    local img_size
    img_size=$(du -h "$OUTPUT_PATH" | cut -f1)
    echo ""
    echo "==> Done!"
    echo "    Image: $OUTPUT_PATH ($img_size)"
    echo ""
    echo "    To boot with Starry:"
    echo "      cargo starry qemu --arch $ARCH"
}

main() {
    parse_args "$@"
    resolve_target_and_output
    check_docker
    build_rootfs
}

main "$@"
