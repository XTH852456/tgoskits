#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_close_range, 0xFFFFFFFFu, 10u, 0);
	int e = errno;
	dprintf(1, "CASE close_range.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
