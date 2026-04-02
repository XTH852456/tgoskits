/* Hand-written contract probe: renameat2(2) missing oldpath -> ENOENT (flags=0). */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef AT_FDCWD
#define AT_FDCWD (-100)
#endif

int main(void)
{
	errno = 0;
	int r = (int)syscall(SYS_renameat2, AT_FDCWD, "/__starryos_probe_renameat2__/old",
			      AT_FDCWD, "/__starryos_probe_renameat2__/new", 0);
	int e = errno;
	dprintf(1, "CASE renameat2.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
