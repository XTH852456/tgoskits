/* Hand-written contract probe: chdir(2) missing absolute path -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = chdir("/__starryos_probe_chdir__/not_there");
	int e = errno;
	dprintf(1, "CASE chdir.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
