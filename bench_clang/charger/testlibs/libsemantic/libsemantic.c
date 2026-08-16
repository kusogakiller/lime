#include "libsemantic.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

static char g_name_buf[64];

SemObj *sem_create(int id) {
    SemObj *o = (SemObj *)malloc(sizeof(SemObj));
    if (!o) return NULL;
    o->id = id;
    o->refcount = 1;
    return o;
}

void sem_destroy(SemObj *obj) {
    if (obj) free(obj);
}

const char *sem_get_name(SemObj *obj) {
    if (!obj) return "none";
    snprintf(g_name_buf, sizeof(g_name_buf), "obj-%d", obj->id);
    return g_name_buf;
}

int sem_take_nullable(SemObj *obj) {
    return obj ? obj->id : -1;
}

int sem_take_nonnull(SemObj *obj) {
    return obj->id;
}

void sem_consume(SemObj *obj) {
    if (obj) free(obj);
}

static SemCb *g_retained_cb = NULL;

void sem_cb_register(SemCb *cb) { g_retained_cb = cb; }
void sem_cb_fire(int value) { if (g_retained_cb && g_retained_cb->on_event) g_retained_cb->on_event(value); }
void sem_cb_unregister(void) { g_retained_cb = NULL; }
void sem_cb_once(SemCb *cb, int value) { if (cb && cb->on_event) cb->on_event(value); }

SemHandle *sem_handle_create(int tag) {
    SemHandle *h = (SemHandle *)malloc(sizeof(SemHandle));
    if (!h) return NULL;
    h->tag = tag;
    h->priv = NULL;
    return h;
}

void sem_handle_close(SemHandle *h) {
    if (h) free(h);
}

SemObj *sem_shared = NULL;

SemObj *sem_make(int id) { return sem_create(id); }
int sem_use(SemObj *obj) { return obj ? obj->id : -1; }
int sem_pick(int which) { return which * 2; }
