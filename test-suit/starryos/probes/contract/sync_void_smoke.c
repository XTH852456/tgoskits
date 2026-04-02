/* Hand-written contract probe: sync(2) void; printable smoke line for oracle/guest. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	sync();
	dprintf(1, "CASE sync.void_smoke ret=0 errno=0 note=handwritten\n");
	return 0;
}
