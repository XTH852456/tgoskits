/* GENERATED — write — template contract_write_zero */
#include <stdio.h>
#include <unistd.h>

int main(void) {
  ssize_t n = write(1, "", 0);
  dprintf(1, "CASE write.write_zero ret=%zd errno=0 note=generated-from-catalog\n", n);
  return 0;
}
