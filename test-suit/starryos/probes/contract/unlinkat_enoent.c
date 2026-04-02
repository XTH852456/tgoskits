/* Hand-written contract probe: unlinkat(2) missing path (AT_FDCWD) -> ENOENT. */
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
	int r = unlinkat(AT_FDCWD, "/__starryos_probe_unlinkat__/nope", 0);
	int e = errno;
	dprintf(1, "CASE unlinkat.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
