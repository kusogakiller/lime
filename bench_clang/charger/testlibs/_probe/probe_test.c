#include <stddef.h>
#include <stdio.h>
#include "abi_probe.h"
int main(){
  printf("LAYOUT %s %zu %zu %d\n", "Bitfield", (size_t)sizeof(struct Bitfield), (size_t)_Alignof(struct Bitfield), (int)1);
  printf(" %zu", (size_t)offsetof(struct Bitfield, normal));
  printf("\n");
  printf("LAYOUT %s %zu %zu %d\n", "Buffer", (size_t)sizeof(struct Buffer), (size_t)_Alignof(struct Buffer), (int)2);
  printf(" %zu", (size_t)offsetof(struct Buffer, len));
  printf(" %zu", (size_t)offsetof(struct Buffer, data));
  printf("\n");
  printf("LAYOUT %s %zu %zu %d\n", "Packed", (size_t)sizeof(struct Packed), (size_t)_Alignof(struct Packed), (int)2);
  printf(" %zu", (size_t)offsetof(struct Packed, a));
  printf(" %zu", (size_t)offsetof(struct Packed, b));
  printf("\n");
  printf("LAYOUT %s %zu %zu %d\n", "AnonUnion", (size_t)sizeof(struct AnonUnion), (size_t)_Alignof(struct AnonUnion), (int)2);
  printf(" %zu", (size_t)offsetof(struct AnonUnion, kind));
  printf(" %zu", (size_t)offsetof(struct AnonUnion, i));
  printf("\n");
  printf("LAYOUT %s %zu %zu %d\n", "CallbackTable", (size_t)sizeof(struct CallbackTable), (size_t)_Alignof(struct CallbackTable), (int)2);
  printf(" %zu", (size_t)offsetof(struct CallbackTable, open));
  printf(" %zu", (size_t)offsetof(struct CallbackTable, close));
  printf("\n");
  return 0;
}
