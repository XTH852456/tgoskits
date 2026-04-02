/* Hand-written contract probe: linkat(2) missing oldpath (AT_FDCWD) -> ENOENT. */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

#ifndef AT_FDCWD
#define AT_FDCWD (-100)
#endif

int main(void)
{
	errno = 0;
	int r = linkat(AT_FDCWD, "/__starryos_probe_linkat__/old", AT_FDCWD,
		       "/__starryos_probe_linkat__/new", 0);
	int e = errno;
	dprintf(1, "CASE linkat.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
