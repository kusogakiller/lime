// Lime Runtime Library
//
// Embedded in the compiler (src/codegen/runtime/) and compiled on-the-fly when
// linking compiled Lime programs into executables. The behaviour (including
// error messages and edge cases) matches the target runtime object
// (test_print/runtime.obj) so that programs behave identically.

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "runtime.h"

// -- Allocation / control flow --

// Allocate `size` bytes (rounded up so zero-size requests get at least 1
// byte). Aborts with a panic message on out-of-memory.
void* runtime_alloc(int64_t size, int64_t align) {
    (void)align;
    if (size < 2) size = 1;
    void* p = malloc((size_t)size);
    if (p == NULL) {
        runtime_panic("runtime_alloc: out of memory");
    }
    return p;
}

void runtime_free(void* p) {
    free(p);
}

// Print an error to stderr and abort. `msg == NULL` is displayed as "(null)".
void runtime_panic(char* msg) {
    if (msg == NULL) msg = "(null)";
    fprintf(stderr, "Lime runtime panic: %s\n", msg);
    abort();
}

// -- Print --
// Print a NUL-terminated string to stdout. NULL is a no-op.
void runtime_print(char* s) {
    if (s == NULL) return;
    fputs(s, stdout);
}

// -- String operations --

// Substring [start, end) using byte offsets, clamped to the string bounds.
// NULL input yields an empty string.
char* runtime_str_slice(char* s, int64_t start, int64_t end) {
    if (s == NULL) {
        char* r = (char*)malloc(1);
        if (r != NULL) r[0] = '\0';
        return r;
    }
    int64_t len = (int64_t)strlen(s);
    int64_t lo = start < 0 ? 0 : start;
    int64_t hi = end < len ? end : len;
    if (hi < lo) hi = lo;
    int64_t n = hi - lo;
    char* r = (char*)malloc((size_t)(n + 1));
    if (r == NULL) {
        runtime_panic("runtime_str_slice: out of memory");
    }
    memcpy(r, s + lo, (size_t)n);
    r[n] = '\0';
    return r;
}

// Concatenate two NUL-terminated strings. NULL operands are treated as "".
char* runtime_str_concat(char* a, char* b) {
    if (a == NULL) a = "";
    if (b == NULL) b = "";
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char* r = (char*)malloc(la + lb + 1);
    if (r == NULL) {
        runtime_panic("runtime_str_concat: out of memory");
    }
    memcpy(r, a, la);
    memcpy(r + la, b, lb);
    r[la + lb] = '\0';
    return r;
}

// -- List helpers --

// Grow a LimeList buffer so capacity doubles (starts at 4) until it exceeds
// the current length. Aborts with a panic message on out-of-memory.
static void grow_list(LimeList* list) {
    int64_t new_cap = list->cap == 0 ? 4 : list->cap * 2;
    while (new_cap <= list->len) {
        new_cap *= 2;
    }
    char* new_data = (char*)realloc(list->data, (size_t)(new_cap * 8));
    if (new_data == NULL) {
        runtime_panic("runtime_list: out of memory");
    }
    list->data = new_data;
    list->cap = new_cap;
}

// UTF-8 decode one sequence starting at s[offset]; returns the code point and
// the number of bytes consumed. Invalid leading bytes are stored raw.
static int64_t decode_utf8(const char* s, int64_t offset, int64_t* consumed) {
    unsigned char b = (unsigned char)s[offset];
    if (b < 0x80) {
        *consumed = 1;
        return (int64_t)b;
    } else if ((b & 0xE0) == 0xC0) {
        *consumed = 2;
        return ((int64_t)(b & 0x1F) << 6)
             | ((unsigned char)s[offset + 1] & 0x3F);
    } else if ((b & 0xF0) == 0xE0) {
        *consumed = 3;
        return ((int64_t)(b & 0x0F) << 12)
             | ((int64_t)((unsigned char)s[offset + 1] & 0x3F) << 6)
             | ((unsigned char)s[offset + 2] & 0x3F);
    } else if ((b & 0xF8) == 0xF0) {
        *consumed = 4;
        return ((int64_t)(b & 0x07) << 18)
             | ((int64_t)((unsigned char)s[offset + 1] & 0x3F) << 12)
             | ((int64_t)((unsigned char)s[offset + 2] & 0x3F) << 6)
             | ((unsigned char)s[offset + 3] & 0x3F);
    }
    *consumed = 1;
    return (int64_t)b;
}

// Decode the string as UTF-8 and return a list of code points.
LimeList runtime_str_chars(char* s) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    if (s == NULL) return list;

    int64_t offset = 0;
    while (s[offset] != '\0') {
        int64_t consumed;
        int64_t cp = decode_utf8(s, offset, &consumed);
        if (list.len >= list.cap) grow_list(&list);
        ((int64_t*)list.data)[list.len] = cp;
        list.len++;
        offset += consumed;
    }
    return list;
}

// Return a list of the raw byte values of the string.
LimeList runtime_str_bytes(char* s) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    if (s == NULL) return list;

    int64_t offset = 0;
    while (s[offset] != '\0') {
        if (list.len >= list.cap) grow_list(&list);
        ((int64_t*)list.data)[list.len] = (int64_t)(unsigned char)s[offset];
        list.len++;
        offset++;
    }
    return list;
}

// -- List operations --

// Construct an empty list.
LimeList runtime_list_empty(void) {
    LimeList list;
    list.data = NULL;
    list.len = 0;
    list.cap = 0;
    return list;
}

// Append an element, growing the buffer as needed.
LimeList runtime_list_add(LimeList list, int64_t elem) {
    if (list.len >= list.cap) grow_list(&list);
    ((int64_t*)list.data)[list.len] = elem;
    list.len++;
    return list;
}

// Replace the element at `index` (no-op when the index is out of bounds).
LimeList runtime_list_set(LimeList list, int64_t index, int64_t elem) {
    if (index >= 0 && index < list.len) {
        ((int64_t*)list.data)[index] = elem;
    }
    return list;
}
