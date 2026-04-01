/* Hand-written contract probe: dup(2) on invalid fd -> EBADF. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = dup(-1);
	int e = errno;
	dprintf(1, "CASE dup.badfd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
