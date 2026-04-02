/* Hand-written contract probe: fchdir(2) invalid fd -> EBADF. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = fchdir(-1);
	int e = errno;
	dprintf(1, "CASE fchdir.badfd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
