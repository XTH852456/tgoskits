/* Hand-written contract probe: write(2) zero-length to stdout. */
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	ssize_t n = write(1, "", 0);
	dprintf(1, "CASE write_stdout.zero_len ret=%zd errno=0 note=handwritten\n", n);
	return 0;
}
