#include "liblayout.h"
int sizes() {
  return (int)(sizeof(Padded)+sizeof(Wrapper)+sizeof(U)+sizeof(Flags));
}
