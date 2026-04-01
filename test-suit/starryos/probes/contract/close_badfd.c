/* Hand-written contract probe: close(2) on invalid fd -> EBADF. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	int r = close(-1);
	int e = errno;
	dprintf(1, "CASE close.badfd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
