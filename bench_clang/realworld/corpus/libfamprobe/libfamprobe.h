#ifndef LIBFAMPROBE_H
#define LIBFAMPROBE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FAMProbe {
    int n;
    char data[];
} FAMProbe;

FAMProbe* fam_alloc(int n);
void fam_free(FAMProbe* p);
void fam_set_n(FAMProbe* p, int n);
int fam_get_n(const FAMProbe* p);
void fam_set_data(FAMProbe* p, int i, char v);
char fam_get_data(const FAMProbe* p, int i);
int fam_sum(const FAMProbe* p);

#ifdef __cplusplus
}
#endif

#endif
