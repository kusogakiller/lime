#include <stddef.h>
#include <stdio.h>
#include "libbitfield.h"
int main(){
  printf("LAYOUT %s %zu %zu %d", "Bitfield", (size_t)sizeof(struct Bitfield), (size_t)_Alignof(struct Bitfield), (int)1);
  printf(" %zu", (size_t)offsetof(struct Bitfield, normal));
  printf("\n");
  return 0;
}
