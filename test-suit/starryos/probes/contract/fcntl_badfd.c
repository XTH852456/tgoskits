/* Hand-written contract probe: fcntl(2) on invalid fd -> EBADF. */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>

int main(void)
{
	errno = 0;
	int r = fcntl(-1, F_GETFD);
	int e = errno;
	dprintf(1, "CASE fcntl.bad_fd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
