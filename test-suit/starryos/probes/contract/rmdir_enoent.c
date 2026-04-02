/* Hand-written contract probe: rmdir(2) missing path -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = rmdir("/__starryos_probe_rmdir__/nope");
	int e = errno;
	dprintf(1, "CASE rmdir.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
