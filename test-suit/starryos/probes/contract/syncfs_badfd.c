/* Hand-written contract probe: syncfs(2) invalid fd -> EBADF. */
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	long r = syscall(SYS_syncfs, -1);
	int e = errno;
	dprintf(1, "CASE syncfs.badfd ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
