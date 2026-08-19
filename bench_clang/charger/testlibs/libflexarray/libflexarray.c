#include "libflexarray.h"

int flex_sum(Buffer *b) {
    int total = 0;
    for (size_t i = 0; i < b->len; i++) {
        total += (unsigned char)b->data[i];
    }
    return total;
}
