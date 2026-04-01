/* Hand-written contract probe: read(2) with count 0 on stdin -> 0. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	ssize_t n = read(0, NULL, 0);
	int e = errno;
	dprintf(1, "CASE read_stdin_zero.zero_count ret=%zd errno=%d note=handwritten\n", n, e);
	return 0;
}
