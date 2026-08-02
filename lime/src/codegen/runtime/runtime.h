#ifndef LIME_RUNTIME_H
#define LIME_RUNTIME_H

#include <stdint.h>

typedef struct {
    char* data;
    int64_t len;
    int64_t cap;
} LimeList;

typedef struct {
    int8_t has_value;
    void* value;
} LimeOption;

typedef struct {
    void* data;
    void* vtable;
} LimeIface;

void* runtime_alloc(int64_t size, int64_t align);
void runtime_free(void* p);
void runtime_panic(char* msg);

void runtime_print(char* s);

char* runtime_str_slice(char* s, int64_t start, int64_t end);
char* runtime_str_concat(char* a, char* b);
LimeList runtime_str_chars(char* s);
LimeList runtime_str_bytes(char* s);

LimeList runtime_list_empty(void);
LimeList runtime_list_add(LimeList list, int64_t elem);
LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem);

#endif
