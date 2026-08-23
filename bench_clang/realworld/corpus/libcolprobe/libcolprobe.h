#ifndef LIBCOLPROBE_H
#define LIBCOLPROBE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ColProbe_ {
    int marker;
} *ColHandle;

ColHandle col_make(void);

#ifdef __cplusplus
}
#endif

#endif
