/* Hand-written contract probe: link(2) missing oldpath -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = link("/__starryos_probe_link__/old", "/__starryos_probe_link__/new");
	int e = errno;
	dprintf(1, "CASE link.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
