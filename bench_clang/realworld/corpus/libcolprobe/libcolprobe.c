#include "libcolprobe.h"
#include <stdlib.h>

ColHandle col_make(void) {
    ColHandle h = (ColHandle)malloc(sizeof(*h));
    h->marker = 41;
    return h;
}
