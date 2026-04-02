/* Hand-written contract probe: symlink(2) parent of linkpath missing -> ENOENT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = symlink("target", "/__starryos_probe_symlink__/noparent/x");
	int e = errno;
	dprintf(1, "CASE symlink.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
