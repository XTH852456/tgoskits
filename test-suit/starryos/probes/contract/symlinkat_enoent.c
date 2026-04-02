/* Hand-written contract probe: symlinkat(2) parent of linkpath missing -> ENOENT. */
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
	int r = symlinkat("t", AT_FDCWD, "/__starryos_probe_symlinkat__/noparent/x");
	int e = errno;
	dprintf(1, "CASE symlinkat.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
