/* GENERATED — read — template contract_read_zero */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
  errno = 0;
  ssize_t n = read(0, NULL, 0);
  dprintf(1, "CASE read.zero_count ret=%zd errno=%d note=generated-from-catalog\n", n, errno);
  return 0;
}
