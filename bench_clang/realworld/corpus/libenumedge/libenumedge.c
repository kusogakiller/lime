#include "libenumedge.h"
int take_enum_u8(enum e_u8 v) { return (int)v; }
enum e_i16 ret_enum_i16(int v) { return (v > 32767) ? (enum e_i16)32767 : (enum e_i16)v; }
