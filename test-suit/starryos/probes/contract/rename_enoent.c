/* Hand-written contract probe: rename(2) missing oldpath -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = rename("/__starryos_probe_rename__/old", "/__starryos_probe_rename__/new");
	int e = errno;
	dprintf(1, "CASE rename.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
