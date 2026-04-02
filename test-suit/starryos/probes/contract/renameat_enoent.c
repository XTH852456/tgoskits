/* Hand-written contract probe: renameat(2) missing oldpath -> ENOENT. */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>

#ifndef AT_FDCWD
#define AT_FDCWD (-100)
#endif

int main(void)
{
	errno = 0;
	int r = renameat(AT_FDCWD, "/__starryos_probe_renameat__/old", AT_FDCWD,
			 "/__starryos_probe_renameat__/new");
	int e = errno;
	dprintf(1, "CASE renameat.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
