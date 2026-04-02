/* Hand-written contract probe: getdents64(2) invalid fd -> EBADF. */
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(void)
{
	char buf[256];
	errno = 0;
	long r = syscall(SYS_getdents64, -1, buf, sizeof(buf));
	int e = errno;
	dprintf(1, "CASE getdents64.badfd ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
