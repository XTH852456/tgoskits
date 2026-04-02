/* Hand-written contract probe: mkdirat(2) missing parent (AT_FDCWD) -> ENOENT. */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <sys/types.h>

#ifndef AT_FDCWD
#define AT_FDCWD (-100)
#endif

int main(void)
{
	errno = 0;
	int r = mkdirat(AT_FDCWD, "/__starryos_probe_mkdirat__/missing/dir", 0777);
	int e = errno;
	dprintf(1, "CASE mkdirat.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
