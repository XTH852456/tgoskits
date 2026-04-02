/* Hand-written contract probe: chroot(2) missing absolute path -> ENOENT (qemu-riscv64 user). */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = chroot("/__starryos_probe_chroot__/not_there");
	int e = errno;
	dprintf(1, "CASE chroot.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
