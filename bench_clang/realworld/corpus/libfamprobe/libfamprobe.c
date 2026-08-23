#include "libfamprobe.h"
#include <stdlib.h>
#include <string.h>

FAMProbe* fam_alloc(int n) {
    FAMProbe* p = (FAMProbe*)malloc(sizeof(FAMProbe) + (size_t)n);
    if (p) { p->n = n; memset(p->data, 0, (size_t)n); }
    return p;
}
void fam_free(FAMProbe* p) { free(p); }
void fam_set_n(FAMProbe* p, int n) { p->n = n; }
int fam_get_n(const FAMProbe* p) { return p->n; }
void fam_set_data(FAMProbe* p, int i, char v) { p->data[i] = v; }
char fam_get_data(const FAMProbe* p, int i) { return p->data[i]; }
int fam_sum(const FAMProbe* p) {
    int s = 0;
    for (int i = 0; i < p->n; i++) s += (unsigned char)p->data[i];
    return s;
}
