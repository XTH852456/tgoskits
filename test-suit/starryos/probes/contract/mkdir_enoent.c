/* Hand-written contract probe: mkdir(2) missing parent dir -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <sys/stat.h>
#include <sys/types.h>

int main(void)
{
	errno = 0;
	int r = mkdir("/__starryos_probe_mkdir__/missing/dir", 0777);
	int e = errno;
	dprintf(1, "CASE mkdir.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
