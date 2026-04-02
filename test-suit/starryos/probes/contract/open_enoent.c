#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>
static const char p[] = "/__starryos_probe_open__/not_there";
int main(void)
{
	errno = 0;
	int r = open(p, O_RDONLY | O_NOCTTY);
	int e = errno;
	dprintf(1, "CASE open.enoent ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
